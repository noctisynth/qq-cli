//! Account-free runtime preparation and once-only native lifecycle.
pub mod default_start;
#[cfg(windows)]
pub mod dispatch;
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub mod generation;
pub mod instance_creation;
pub mod launch;
#[cfg(windows)]
pub(crate) mod observation;
mod operation_lock;
pub(crate) use operation_lock::acquire as lifecycle_lock;
pub mod ports;
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub mod remove;
#[cfg(windows)]
pub mod retention;
#[cfg(windows)]
pub mod run_job;
pub mod runtime_config;
pub mod runtime_store;
#[cfg(windows)]
pub mod start_engine;
pub mod start_intent;
pub mod startup;
#[cfg(windows)]
pub mod stop;
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub mod version_intent;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod macos;
