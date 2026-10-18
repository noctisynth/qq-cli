use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Parser, error::ErrorKind};
use qq_cli::{app, cli::Cli, error::AppError, output::Envelope};

fn main() -> ExitCode {
    let result = match Cli::try_parse() {
        Ok(cli) if matches!(cli.command, qq_cli::cli::Command::Event { .. }) => {
            return streaming(cli);
        }
        Ok(cli) => app::execute(cli),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return if error.print().is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(6)
            };
        }
        Err(_) => Envelope::failure(AppError::invalid()),
    };
    let mut stdout = io::stdout().lock();
    if serde_json::to_writer(&mut stdout, &result).is_err()
        || stdout.write_all(b"\n").is_err()
        || stdout.flush().is_err()
    {
        return ExitCode::from(6);
    }
    ExitCode::from(result.exit_code())
}

fn streaming(cli: Cli) -> ExitCode {
    use qq_cli::watch::Sink;
    use std::sync::{Arc, atomic::AtomicBool};
    use std::time::{Duration, Instant};
    let cancellation = qq_cli::stream_output::cancellation();
    let flag = cancellation
        .as_ref()
        .map(Arc::clone)
        .unwrap_or_else(|_| Arc::new(AtomicBool::new(false)));
    let Ok(mut output) = qq_cli::stream_output::Output::new(io::stdout(), flag.clone()) else {
        return ExitCode::from(6);
    };
    let result = cancellation.and_then(|_| app::execute_watch(&cli, &mut output, &flag));
    let code = match result {
        Ok(code) => code,
        Err(error) => {
            let code = error.exit_code;
            if !output.started() {
                let envelope = Envelope::failure(error);
                let value = serde_json::to_value(envelope).unwrap_or_default();
                if output
                    .record(&value, Instant::now() + Duration::from_secs(1))
                    .is_err()
                {
                    return ExitCode::from(6);
                }
            }
            code
        }
    };
    ExitCode::from(code)
}
