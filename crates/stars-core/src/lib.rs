#![forbid(unsafe_code)]
//! Authoritative game rules and state contracts for the Stars! remake.
//!
//! This crate deliberately has no user-interface, operating-system, clock,
//! filesystem or threading dependencies. Gameplay arithmetic, validation and
//! deterministic state transitions belong here and must remain independent of
//! the desktop front end.
//!
//! PR3 establishes only the package boundary and stable identifier primitive.
//! Game state, orders, views and turn processing arrive in PR4.

/// Version of the public core contract.
///
/// Bump this when a state or order contract changes incompatibly. It is not a
/// save-file format version; persistence is introduced in PR16.
pub const CORE_API_VERSION: u32 = 1;

/// A stable identifier for an entity in the game state.
///
/// Identifiers are explicit integers rather than hash values, allocation
/// addresses or iteration positions, so serialized state and replay do not
/// depend on process layout. Construction is intentionally narrow; gameplay
/// code allocates identifiers through the owning state in PR4.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(u64);

impl EntityId {
    /// Creates an identifier from its stable numeric representation.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the stable numeric representation.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "entity-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{CORE_API_VERSION, EntityId};

    #[test]
    fn entity_ids_round_trip_and_order_numerically() {
        let first = EntityId::from_raw(7);
        let second = EntityId::from_raw(11);

        assert_eq!(first.raw(), 7);
        assert_eq!(first.to_string(), "entity-7");
        assert!(first < second);
    }

    #[test]
    fn core_contract_version_is_explicit() {
        assert_eq!(CORE_API_VERSION, 1);
    }
}
