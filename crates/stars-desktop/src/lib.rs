#![forbid(unsafe_code)]
//! Offline desktop application foundation for the Stars! remake.
//!
//! The desktop package owns the window, rendering, local I/O and the
//! background worker. Gameplay contracts stay in `stars-core`; AI decisions
//! stay in `stars-ai`.

pub mod app;
pub mod preview;
pub mod shell;
pub mod turn_worker;
pub mod worker;

pub use app::StarsApp;
