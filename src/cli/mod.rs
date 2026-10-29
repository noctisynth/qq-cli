//! Command contracts, validation, discovery and process output.
pub mod account;
pub mod args;
pub mod contact;
pub mod group;
pub mod message_ops;
pub mod registry;
pub mod schema;
pub mod skill;
pub mod stream_output;
pub mod validation;
mod voice;
pub mod write;

mod input_schema;

mod rules;

/// Reuse the actual built Clap tree rather than rebuilding its large derive stack inside app frames.
fn command() -> clap::Command {
    use clap::CommandFactory;
    static COMMAND: std::sync::OnceLock<clap::Command> = std::sync::OnceLock::new();
    COMMAND
        .get_or_init(|| {
            let mut command = args::Cli::command();
            command.build();
            command
        })
        .clone()
}
/// Initialize the same immutable command tree at the process entry, before application orchestration.
pub fn parse() -> std::result::Result<args::Cli, clap::Error> {
    use clap::FromArgMatches;
    command()
        .try_get_matches()
        .and_then(|mut matches| args::Cli::from_arg_matches_mut(&mut matches))
}
