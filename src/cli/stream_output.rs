//! Process-scoped bounded output worker. A timed-out writer is never reused.
//! The CLI exits after failure; it never joins a worker blocked in OS stdout I/O.
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use crate::core::contracts::event::Event;
use crate::core::error::{AppError, Result};
use crate::messaging::watch::Sink;

struct Record {
    line: Vec<u8>,
    acknowledgement: mpsc::SyncSender<Result<()>>,
}
pub struct Output {
    sender: Option<mpsc::SyncSender<Record>>,
    cancel: Arc<AtomicBool>,
    started: bool,
    failed: bool,
}

impl Output {
    pub fn new(mut writer: impl Write + Send + 'static, cancel: Arc<AtomicBool>) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Record>(1);
        std::thread::Builder::new()
            .name("qq-cli-output".into())
            .spawn(move || {
                while let Ok(record) = receiver.recv() {
                    let result = writer
                        .write_all(&record.line)
                        .and_then(|()| writer.flush())
                        .map_err(|_| output_error());
                    let failed = result.is_err();
                    if record.acknowledgement.send(result).is_err() || failed {
                        break;
                    }
                }
            })
            .map_err(|_| output_error())?;
        Ok(Self {
            sender: Some(sender),
            cancel,
            started: false,
            failed: false,
        })
    }
    pub fn failure(
        &mut self,
        value: &crate::core::contracts::reply::Failure,
        deadline: Instant,
    ) -> Result<()> {
        let result = self.send(value, deadline, false);
        if result.is_err() {
            self.failed = true;
            self.sender.take();
        }
        result
    }
    pub fn started(&self) -> bool {
        self.started
    }
    fn send(
        &mut self,
        value: &impl serde::Serialize,
        deadline: Instant,
        summary: bool,
    ) -> Result<()> {
        if self.failed {
            return Err(output_error());
        }
        let mut line = serde_json::to_vec(value).map_err(|_| AppError::protocol())?;
        if line.len() > 4 * 1024 * 1024 {
            return Err(AppError::new(
                "EVENT_STREAM_LIMIT",
                "Encoded event exceeded the line budget.",
                4,
            ));
        }
        line.push(b'\n');
        let (acknowledgement, receipt) = mpsc::sync_channel(1);
        let mut record = Record {
            line,
            acknowledgement,
        };
        loop {
            self.check(deadline, summary)?;
            match self
                .sender
                .as_ref()
                .ok_or_else(output_error)?
                .try_send(record)
            {
                Ok(()) => {
                    self.started = true;
                    break;
                }
                Err(mpsc::TrySendError::Full(pending)) => {
                    record = pending;
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(mpsc::TrySendError::Disconnected(_)) => return Err(output_error()),
            }
        }
        loop {
            self.check(deadline, summary)?;
            match receipt.recv_timeout(
                Duration::from_millis(25).min(deadline.saturating_duration_since(Instant::now())),
            ) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(output_error()),
            }
        }
    }
    fn check(&self, deadline: Instant, summary: bool) -> Result<()> {
        if Instant::now() >= deadline {
            return Err(AppError::new(
                "OUTPUT_DEADLINE",
                "Output did not finish before its deadline; observation stopped.",
                6,
            ));
        }
        if !summary && self.cancel.load(Ordering::Relaxed) {
            return Err(AppError::new(
                "CANCELLED",
                "Event output was cancelled.",
                130,
            ));
        }
        Ok(())
    }
}
impl Sink for Output {
    fn record(&mut self, value: &Event, deadline: Instant) -> Result<()> {
        let result = self.send(value, deadline, matches!(value, Event::Summary { .. }));
        if result.is_err() {
            self.failed = true;
            self.sender.take();
        }
        result
    }
}
fn output_error() -> AppError {
    AppError::new(
        "OUTPUT_IO",
        "Event output was interrupted; no further records were attempted.",
        6,
    )
}

pub fn cancellation() -> Result<Arc<AtomicBool>> {
    let flag = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&flag);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed)).map_err(|_| {
        AppError::new(
            "SIGNAL_INIT",
            "Cancellation handler could not be installed.",
            6,
        )
    })?;
    Ok(flag)
}
