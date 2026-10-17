mod cli;
mod output;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Parser, error::ErrorKind};

use cli::Cli;
use output::Envelope;

fn main() -> ExitCode {
    let envelope = match Cli::try_parse() {
        Ok(cli) => Envelope::not_implemented(cli.command.name(), cli.command.request_id()),
        Err(error) => {
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                return match error.print() {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(_) => ExitCode::from(6),
                };
            }
            // Clap diagnostics can echo supplied values, including secrets or message text.
            Envelope::invalid_arguments()
        }
    };

    let mut stdout = io::stdout().lock();
    let result = serde_json::to_writer(&mut stdout, &envelope)
        .map_err(io::Error::other)
        .and_then(|()| stdout.write_all(b"\n"))
        .and_then(|()| stdout.flush());
    if result.is_err() {
        return ExitCode::from(6);
    }
    ExitCode::from(envelope.exit_code())
}
