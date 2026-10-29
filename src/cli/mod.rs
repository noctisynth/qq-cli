//! Command contracts, validation, discovery and process output.
pub mod account;
pub mod album;
pub mod args;
pub mod assets;
pub mod contact;
pub mod files;
pub mod flash;
pub mod group;
pub mod message_ops;
pub mod registry;
pub mod schema;
pub mod skill;
pub mod social;
pub mod stream_output;
pub mod validation;
mod voice;
pub mod write;

mod input_schema;

mod rules;

/// Reuse the actual built Clap tree rather than rebuilding its large derive stack inside app frames.
fn command() -> clap::Command {
    static COMMAND: std::sync::OnceLock<clap::Command> = std::sync::OnceLock::new();
    COMMAND
        .get_or_init(|| {
            // Windows main threads have a small default stack. Clap's generated tree
            // builder keeps many temporaries alive in debug builds; isolate that one
            // initialization instead of moving application or runtime work to a worker.
            #[cfg(windows)]
            {
                std::thread::Builder::new()
                    .name("cli-definition".into())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(build_command)
                    .expect("command definition worker")
                    .join()
                    .expect("command definition")
            }
            #[cfg(not(windows))]
            {
                build_command()
            }
        })
        .clone()
}
fn build_command() -> clap::Command {
    use clap::CommandFactory;
    let mut command = args::Cli::command();
    command.build();
    command
}

/// Initialize the same immutable command tree at the process entry, before application orchestration.
pub fn parse() -> std::result::Result<args::Cli, clap::Error> {
    use clap::FromArgMatches;
    command()
        .try_get_matches()
        .and_then(|mut matches| args::Cli::from_arg_matches_mut(&mut matches))
}
