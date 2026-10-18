#![forbid(unsafe_code)]
//! Headless utilities for the Stars! remake.
//!
//! This binary checks that the headless tooling package can use the core and
//! AI packages without pulling in the desktop front end. It performs no
//! gameplay simulation yet; headless scenarios arrive with later PRs.

use stars_ai::{TURINDRONES_NAME, core_contract_version};

fn main() {
    let core_version = core_contract_version();
    let entity = stars_core::EntityId::from_raw(1);

    println!("stars-tools foundation check");
    println!("core API version: {core_version}");
    println!("sample entity: {entity}");
    println!("opponent: {TURINDRONES_NAME}");
}
