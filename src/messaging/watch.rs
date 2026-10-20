//! Subscription orchestration. Sources and sinks expose no chat write operations.
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::core::domain::{Conversation, Scope};
use crate::core::error::{AppError, Result};
use crate::messaging::events::Watch;
use crate::messaging::session::Session;
use crate::storage::references::References;
use crate::transport::client::Backend;
use crate::transport::websocket::{Connection, Frame};
mod recovery;

pub trait Source {
    fn connect(&mut self, deadline: Instant) -> Result<()>;
    fn receive(&mut self) -> Result<Frame>;
    fn disconnect(&mut self);
}

/// Implementations must honor deadline themselves when underlying output can block.
pub trait Sink {
    fn record(&mut self, value: &Value, deadline: Instant) -> Result<()>;
}

pub struct SocketSource<'a> {
    pub endpoint: &'a str,
    pub token: &'a str,
    connection: Option<Connection>,
    #[cfg(windows)]
    ownership: Option<(
        &'a crate::native::job::Job,
        &'a crate::native::process::Pinned,
    )>,
}
impl<'a> SocketSource<'a> {
    pub fn new(endpoint: &'a str, token: &'a str) -> Self {
        Self {
            endpoint,
            token,
            connection: None,
            #[cfg(windows)]
            ownership: None,
        }
    }
    /// Native capabilities come from authenticated lifecycle records, not arbitrary PIDs.
    /// Every connect/reconnect checks the actual socket before credentials are sent.
    #[cfg(windows)]
    pub fn owned(
        endpoint: &'a str,
        token: &'a str,
        job: &'a crate::native::job::Job,
        owner: &'a crate::native::process::Pinned,
    ) -> Self {
        Self {
            endpoint,
            token,
            connection: None,
            ownership: Some((job, owner)),
        }
    }
}
impl Source for SocketSource<'_> {
    fn connect(&mut self, deadline: Instant) -> Result<()> {
        // A failed replacement must not leave the previous connection usable.
        self.disconnect();
        #[cfg(windows)]
        if let Some((job, owner)) = self.ownership {
            self.connection = Some(Connection::connect_owned(
                self.endpoint,
                self.token,
                job,
                owner,
                deadline,
            )?);
            return Ok(());
        }
        self.connection = Some(Connection::connect(self.endpoint, self.token, deadline)?);
        Ok(())
    }
    fn receive(&mut self) -> Result<Frame> {
        self.connection
            .as_mut()
            .ok_or_else(AppError::protocol)?
            .receive()
    }
    fn disconnect(&mut self) {
        self.connection.take();
    }
}

/// Atomic record encoding; a plain blocking writer still needs an OS deadline adapter.
pub fn write_record(writer: &mut impl Write, value: &Value, deadline: Instant) -> Result<()> {
    check_deadline(deadline)?;
    let mut line = serde_json::to_vec(value).map_err(|_| AppError::protocol())?;
    if line.len() > 4 * 1024 * 1024 {
        return Err(AppError::new(
            "EVENT_STREAM_LIMIT",
            "Encoded event exceeded the line budget.",
            4,
        ));
    }
    line.push(b'\n');
    writer
        .write_all(&line)
        .and_then(|()| writer.flush())
        .map_err(|_| {
            AppError::new(
                "OUTPUT_IO",
                "Event output was interrupted; observation stopped.",
                6,
            )
        })?;
    check_deadline(deadline)
}

pub struct Options {
    pub conversation: Option<Conversation>,
    pub limit: u16,
    pub deadline: Instant,
    pub self_messages: Option<bool>,
}

pub fn run<B: Backend>(
    session: &Session<'_, B>,
    references: &References,
    source: &mut impl Source,
    sink: &mut impl Sink,
    options: Options,
    cancel: &AtomicBool,
) -> Result<u8> {
    run_resuming(session, references, source, sink, options, cancel, None)
}
pub fn run_resuming<B: Backend>(
    session: &Session<'_, B>,
    references: &References,
    source: &mut impl Source,
    sink: &mut impl Sink,
    options: Options,
    cancel: &AtomicBool,
    resume: Option<&str>,
) -> Result<u8> {
    let result = run_inner(session, references, source, sink, options, cancel, resume);
    source.disconnect();
    result
}

fn run_inner<B: Backend>(
    session: &Session<'_, B>,
    references: &References,
    source: &mut impl Source,
    sink: &mut impl Sink,
    options: Options,
    cancel: &AtomicBool,
    resume: Option<&str>,
) -> Result<u8> {
    session.authorize(options.conversation.as_ref(), false)?;
    // Policy rejection must precede HTTP identity probes and opening the socket.
    if !session.config.permissions.events {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "The local policy does not permit event observation.",
            4,
        ));
    }
    check_deadline(options.deadline)?;
    if !(1..=1000).contains(&options.limit) {
        return Err(AppError::invalid());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::new(
            "CANCELLED",
            "Event observation was cancelled before connecting.",
            130,
        ));
    }
    let info = session.verify(options.deadline)?;
    let scope = Scope {
        instance_id: session.instance.id.clone(),
        account: info.account,
    };
    let mut watch = Watch::new(
        scope.clone(),
        session.instance.run_id.clone(),
        options.conversation.clone(),
        session.config,
        references,
        options.limit,
        options.deadline,
    )?;
    if let Some(token) = resume {
        watch.restore(token)?;
    }
    let verify = || {
        let observed = session.verify(options.deadline)?;
        if observed.account != scope.account {
            return Err(AppError::new(
                "ACCOUNT_CHANGED",
                "Event identity changed; observation must stop.",
                4,
            ));
        }
        Ok(())
    };
    // The initial observation is the pre-connect identity check. No domain operation
    // happens between it and connect, so probing the same identity again adds no guard.
    source.connect(options.deadline)?;
    verify()?;
    let ready = watch.ready(options.self_messages);
    record(&mut watch, ready, sink, options.deadline)?;
    let mut recovery_error = None;
    if resume.is_some() {
        sink.record(&watch.gap("checkpoint_resume"), options.deadline)?;
        recovery_error =
            recovery::fill(session, references, &mut watch, sink, &options, cancel).err();
        if recovery_error
            .as_ref()
            .is_some_and(|e| e.code == "OUTPUT_IO")
        {
            return Err(recovery_error.take().ok_or_else(AppError::protocol)?);
        }
    }
    let mut reconnects = 0_u8;
    let mut last_check = Instant::now();
    let (reason, error, exit) = loop {
        if let Some(error) = recovery_error.take() {
            if error.code == "OUTPUT_IO" {
                return Err(error);
            }
            break ("recovery_failed", Some(error.code), error.exit_code);
        }
        if cancel.load(Ordering::Relaxed) {
            break ("cancelled", None, 130);
        }
        if watch.full() {
            break ("limit", None, 0);
        }
        if Instant::now() >= options.deadline {
            break ("deadline", None, 0);
        }
        if last_check.elapsed() >= Duration::from_secs(5) {
            if let Err(error) = verify() {
                break ("identity_check_failed", Some(error.code), error.exit_code);
            }
            last_check = Instant::now();
        }
        let received = source.receive();
        if cancel.load(Ordering::Relaxed) {
            break ("cancelled", None, 130);
        }
        if Instant::now() >= options.deadline {
            break ("deadline", None, 0);
        }
        match received {
            Ok(Frame::Text(frame)) => match watch.ingest(&frame) {
                Ok(Some(event)) => record(&mut watch, event, sink, options.deadline)?,
                Ok(None) => (),
                Err(error) => break ("event_error", Some(error.code), error.exit_code),
            },
            Ok(Frame::Idle | Frame::Control) => (),
            Err(error) if error.code == "DEADLINE" => break ("deadline", None, 0),
            disconnected @ (Ok(Frame::Closed) | Err(_)) => {
                let error = disconnected.err();
                if error
                    .as_ref()
                    .is_some_and(|error| error.code != "WS_TRANSPORT")
                {
                    let error = error.ok_or_else(AppError::protocol)?;
                    break ("transport_error", Some(error.code), error.exit_code);
                }
                source.disconnect();
                sink.record(&watch.gap("connection_interrupted"), options.deadline)?;
                if reconnects >= 2 {
                    break ("reconnect_limit", None, 7);
                }
                reconnects += 1;
                let delay = Duration::from_millis(250 * u64::from(reconnects));
                if options.deadline.saturating_duration_since(Instant::now()) <= delay {
                    break ("deadline", None, 0);
                }
                // Short slices keep cancellation responsive during bounded backoff.
                let until = Instant::now() + delay;
                while Instant::now() < until && !cancel.load(Ordering::Relaxed) {
                    std::thread::sleep(
                        Duration::from_millis(25)
                            .min(until.saturating_duration_since(Instant::now())),
                    );
                }
                if cancel.load(Ordering::Relaxed) {
                    break ("cancelled", None, 130);
                }
                if let Err(error) = verify()
                    .and_then(|()| source.connect(options.deadline))
                    .and_then(|()| verify())
                {
                    break ("reconnect_failed", Some(error.code), error.exit_code);
                }
                if options.conversation.is_some()
                    && let Err(error) =
                        recovery::fill(session, references, &mut watch, sink, &options, cancel)
                {
                    if error.code == "OUTPUT_IO" {
                        return Err(error);
                    }
                    break ("recovery_failed", Some(error.code), error.exit_code);
                }
                last_check = Instant::now();
            }
        }
    };
    let mut summary = watch.summary(reason);
    if let Some(code) = error {
        summary["errorCode"] = serde_json::json!(code);
    }
    // Summary delivery after deadline must be given a bounded output-only grace period.
    record(
        &mut watch,
        summary,
        sink,
        Instant::now() + Duration::from_secs(1),
    )?;
    Ok(exit)
}

fn record(
    watch: &mut Watch<'_>,
    mut value: Value,
    sink: &mut impl Sink,
    deadline: Instant,
) -> Result<()> {
    watch.attach_checkpoint(&mut value)?;
    sink.record(&value, deadline)
}

fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(AppError::new(
            "DEADLINE",
            "Event output reached its deadline.",
            3,
        ));
    }
    Ok(())
}
