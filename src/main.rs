use std::io::{self, Write};
use std::process::ExitCode;

use clap::error::ErrorKind;
use qq_cli::{app, cli::args::Cli, core::error::AppError, core::output::Envelope};

fn main() -> ExitCode {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if let Some(code) = qq_cli::storage::credentials::keychain_authorization_worker() {
        return ExitCode::from(code);
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if let Some(code) = qq_cli::napcat::install::macos::worker_entry() {
        return ExitCode::from(code);
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if let Some(code) = qq_cli::native::macos_copy_worker::worker_entry() {
        return ExitCode::from(code);
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if let Some(code) = qq_cli::native::macos_signature::worker_entry() {
        return ExitCode::from(code);
    }
    let result = match qq_cli::cli::parse() {
        Ok(cli)
            if matches!(
                cli.command,
                qq_cli::cli::args::Command::Event { .. }
                    | qq_cli::cli::args::Command::Api {
                        command: qq_cli::cli::args::ApiCommand::Stream(_)
                    }
            ) =>
        {
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
    use std::sync::{Arc, atomic::AtomicBool};
    use std::time::{Duration, Instant};
    let cancellation = qq_cli::cli::stream_output::cancellation();
    let flag = cancellation
        .as_ref()
        .map(Arc::clone)
        .unwrap_or_else(|_| Arc::new(AtomicBool::new(false)));
    let Ok(mut output) = qq_cli::cli::stream_output::Output::new(io::stdout(), flag.clone()) else {
        return ExitCode::from(6);
    };
    let result = cancellation.and_then(|_| match &cli.command {
        qq_cli::cli::args::Command::Api {
            command: qq_cli::cli::args::ApiCommand::Stream(_),
        } => app::execute_api_stream(&cli, &mut output, &flag),
        _ => app::execute_watch(&cli, &mut output, &flag),
    });
    let code = match result {
        Ok(code) => code,
        Err(error) => {
            let code = error.exit_code;
            if !output.started() {
                let value = qq_cli::core::contracts::reply::Failure::new(error);
                if output
                    .failure(&value, Instant::now() + Duration::from_secs(1))
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
