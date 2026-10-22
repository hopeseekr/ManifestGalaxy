#![forbid(unsafe_code)]
//! Turindrones opponent support for the Stars! remake.
//!
//! The AI crate may depend on `stars-core`, but it must consume only the
//! player-visible view and its own serializable memory. PR3 adds the package
//! boundary and a stable opponent name. Planning and memory arrive in PR14.

use stars_core::CORE_API_VERSION;

/// Canonical display name of the built-in computer opponent.
pub const TURINDRONES_NAME: &str = "Turindrones";

/// Returns the core contract version this opponent package was built against.
#[must_use]
pub const fn core_contract_version() -> u32 {
    CORE_API_VERSION
}

#[cfg(test)]
mod tests {
    use super::{TURINDRONES_NAME, core_contract_version};

    #[test]
    fn opponent_uses_the_current_core_contract() {
        assert_eq!(TURINDRONES_NAME, "Turindrones");
        assert_eq!(core_contract_version(), stars_core::CORE_API_VERSION);
    }
}
