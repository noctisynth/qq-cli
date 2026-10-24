//! Original thin Framework bridge. Embedding does not authorize installing or executing it.
//! The lifecycle must verify the dedicated QQ copy, paths and upstream version first.
pub const BRIDGE_SOURCE: &str = include_str!("bridge.cjs");
pub const ENTRY_SOURCE: &str = include_str!("entry.cjs");

mod package;
mod preparation;
mod publication;
pub use package::PackageEdit;
pub use preparation::Prepared;
pub(crate) use preparation::layout_directories;
pub use publication::AdapterPlan;
