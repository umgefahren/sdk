pub mod cache;
pub mod dfinity;

static mut DFX_VERSION: Option<String> = None;
/// Returns the version of DFX that was built.
/// In debug, add a timestamp of the upstream compilation at the end of version to ensure all
/// debug runs are unique (and cached uniquely).
/// That timestamp is taken from the DFX_TIMESTAMP_DEBUG_MODE_ONLY env var that is set in
/// Nix.
pub fn dfx_version() -> &'static str {
    unsafe {
        match &DFX_VERSION {
            Some(x) => x.as_str(),
            None => {
                let version = env!("CARGO_PKG_VERSION");

                if is_debug() {
                    DFX_VERSION = Some(format!("{}-debug", version,));
                }

                dfx_version()
            }
        }
    }
}

#[cfg(debug_assertions)]
pub(super) fn is_debug() -> bool {
    true
}

#[cfg(not(debug_assertions))]
pub(super) fn is_debug() -> bool {
    false
}
