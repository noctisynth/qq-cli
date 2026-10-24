//! Apple Silicon application preparation: authenticated source, owned copies and signing.
//! Runtime orchestration lives in runtime::macos; SDK/unsafe capabilities remain native.
pub mod bundle;
pub mod signing;
pub mod source;

/// Dispatch private installation workers before public CLI parsing.
/// Standalone verification hosts must register the same entry point.
pub fn worker_entry() -> Option<u8> {
    for worker in [
        source::image::release::worker_entry as fn() -> Option<u8>,
        source::image::attach::worker_entry,
        bundle::supervisor_entry,
        signing::worker_entry,
    ] {
        if let Some(code) = worker() {
            return Some(code);
        }
    }
    None
}
