//! Designed, explicit and versioned random-number streams.
//!
//! The original game's RNG algorithm, seed and consumption order are not
//! recovered (`RULE-RNG-001`, `GAP-RNG-001`). This module therefore provides a
//! reproducible remake RNG and labels it as a designed implementation choice,
//! not as original behaviour.
//!
//! Streams are separated by a stable [`RngStreamKey`] built from an optional
//! player identifier and a subsystem. Each stream stores its own draw counter,
//! so consuming a draw in one subsystem cannot shift another subsystem. Stream
//! state is serialized with the game state. Nothing here consults the clock,
//! a thread identifier, a hash map or a platform random source.

use std::collections::BTreeMap;
use std::fmt;

use crate::model::{
    CanonicalEncode, CanonicalReader, PlayerId, StateDecodeError, StateDecodeErrorKind,
};

/// Version of the designed RNG algorithm and stream derivation.
///
/// A state that records a different algorithm version is rejected rather than
/// silently reinterpreted.
pub const RNG_ALGORITHM_VERSION: u16 = 1;

/// Stable subsystem names used to separate independent random streams.
///
/// These are contract names, not evidence that the original game used separate
/// streams or these categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RngSubsystem {
    /// Turn setup and administrative shuffling.
    TurnSetup,
    /// Computer-opponent planning (reserved for PR14).
    AiPlanning,
    /// Fleet movement (reserved for PR10).
    Movement,
    /// Combat rounds (reserved for PR12).
    Combat,
    /// Mining and mineral events (reserved for PR8).
    Mining,
    /// Research and technology events (reserved for PR9).
    Research,
    /// Random events and artifact effects (reserved for a later PR).
    RandomEvents,
}

impl RngSubsystem {
    /// Stable encoded value. Do not renumber.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::TurnSetup => 1,
            Self::AiPlanning => 2,
            Self::Movement => 3,
            Self::Combat => 4,
            Self::Mining => 5,
            Self::Research => 6,
            Self::RandomEvents => 7,
        }
    }

    /// Decodes a stable value written by [`Self::code`].
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::TurnSetup),
            2 => Some(Self::AiPlanning),
            3 => Some(Self::Movement),
            4 => Some(Self::Combat),
            5 => Some(Self::Mining),
            6 => Some(Self::Research),
            7 => Some(Self::RandomEvents),
            _ => None,
        }
    }

    /// Human-readable contract name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TurnSetup => "turn-setup",
            Self::AiPlanning => "ai-planning",
            Self::Movement => "movement",
            Self::Combat => "combat",
            Self::Mining => "mining",
            Self::Research => "research",
            Self::RandomEvents => "random-events",
        }
    }
}

/// Stable identifier for one independent random stream.
///
/// The global stream has no player. Player-specific streams cannot collide with
/// each other because the player identifier is part of the key and of the
/// encoded representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RngStreamKey {
    /// Owning player, or `None` for a game-wide stream.
    pub player: Option<PlayerId>,
    /// Subsystem that consumes the stream.
    pub subsystem: RngSubsystem,
}

impl RngStreamKey {
    /// Creates a stream key.
    #[must_use]
    pub const fn new(player: Option<PlayerId>, subsystem: RngSubsystem) -> Self {
        Self { player, subsystem }
    }

    /// Creates a game-wide stream key.
    #[must_use]
    pub const fn global(subsystem: RngSubsystem) -> Self {
        Self::new(None, subsystem)
    }

    /// Creates a player-specific stream key.
    #[must_use]
    pub const fn for_player(player: PlayerId, subsystem: RngSubsystem) -> Self {
        Self::new(Some(player), subsystem)
    }

    /// Deterministic stream selector mixed into the root seed.
    ///
    /// The game-wide stream uses the reserved selector input 0; player streams
    /// use the player identifier plus one, so no player stream shares an input
    /// with the global stream; only the unreachable identifier `u64::MAX`
    /// could collide with it. This is an integer mix over the stable key
    /// fields; it does not use the
    /// standard library's hasher, whose output is not part of this contract.
    #[must_use]
    pub const fn selector(self) -> u64 {
        let player = match self.player {
            Some(player) => player.raw().wrapping_add(1),
            None => 0,
        };
        mix64(
            player
                ^ 0x9e37_79b9_7f4a_7c15
                ^ (self.subsystem.code() as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9),
        )
    }
}

/// State of one independent random stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RngStreamState {
    /// Number of values already produced by this stream.
    pub draws: u64,
}

/// Versioned random state stored in the authoritative game state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RngState {
    algorithm_version: u16,
    seed: u64,
    streams: BTreeMap<RngStreamKey, RngStreamState>,
}

impl RngState {
    /// Creates an empty state with an explicit root seed.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            algorithm_version: RNG_ALGORITHM_VERSION,
            seed,
            streams: BTreeMap::new(),
        }
    }

    /// Root seed recorded in the state.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Version of the algorithm that produced this state.
    #[must_use]
    pub const fn algorithm_version(&self) -> u16 {
        self.algorithm_version
    }

    /// Number of values already drawn from a stream.
    #[must_use]
    pub fn draws(&self, key: RngStreamKey) -> u64 {
        self.streams.get(&key).map_or(0, |state| state.draws)
    }

    /// Produces the next value from a stream and advances only that stream.
    ///
    /// The returned value depends on the root seed, the stable stream key and
    /// the stream's draw counter. Wrapping integer arithmetic is intentional
    /// and identical on every platform.
    pub fn draw_u64(&mut self, key: RngStreamKey) -> u64 {
        let stream = self.streams.entry(key).or_default();
        let value =
            mix64(self.seed ^ key.selector() ^ stream.draws.wrapping_mul(0x94d0_49bb_1331_11eb));
        stream.draws = stream.draws.wrapping_add(1);
        value
    }

    /// Returns the sorted stream keys currently recorded.
    ///
    /// The order is deterministic because the map is ordered by stable key.
    #[must_use]
    pub fn stream_keys(&self) -> Vec<RngStreamKey> {
        self.streams.keys().copied().collect()
    }

    pub(crate) fn encode_canonical(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.algorithm_version.to_le_bytes());
        out.extend_from_slice(&self.seed.to_le_bytes());
        encode_len(out, self.streams.len());
        for (key, state) in &self.streams {
            match key.player {
                Some(player) => {
                    out.push(1);
                    player.encode_canonical(out);
                }
                None => out.push(0),
            }
            out.push(key.subsystem.code());
            out.extend_from_slice(&state.draws.to_le_bytes());
        }
    }

    pub(crate) fn decode_canonical(
        reader: &mut CanonicalReader<'_>,
    ) -> Result<Self, StateDecodeError> {
        let algorithm_version = reader.read_u16("rng.algorithm_version")?;
        if algorithm_version != RNG_ALGORITHM_VERSION {
            return Err(reader.error(StateDecodeErrorKind::UnsupportedRngVersion(
                algorithm_version,
            )));
        }
        let seed = reader.read_u64("rng.seed")?;
        let count = reader.read_len("rng.streams")?;
        let mut streams = BTreeMap::new();
        for _ in 0..count {
            let has_player = reader.read_bool("rng.stream.player")?;
            let player = if has_player {
                Some(PlayerId::decode_canonical(reader)?)
            } else {
                None
            };
            let subsystem_code = reader.read_u8("rng.stream.subsystem")?;
            let subsystem = RngSubsystem::from_code(subsystem_code).ok_or_else(|| {
                reader.error(StateDecodeErrorKind::InvalidTag {
                    field: "rng.stream.subsystem",
                    value: subsystem_code,
                })
            })?;
            let draws = reader.read_u64("rng.stream.draws")?;
            let key = RngStreamKey::new(player, subsystem);
            if streams.insert(key, RngStreamState { draws }).is_some() {
                return Err(reader.error(StateDecodeErrorKind::DuplicateValue("rng.stream")));
            }
        }

        Ok(Self {
            algorithm_version,
            seed,
            streams,
        })
    }
}

impl fmt::Display for RngState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "rng v{} seed {} ({} streams)",
            self.algorithm_version,
            self.seed,
            self.streams.len()
        )
    }
}

fn encode_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("collection length exceeds canonical encoding limit");
    out.extend_from_slice(&len.to_le_bytes());
}

/// SplitMix64 finalizer with wrapping arithmetic.
///
/// This is a designed deterministic mixer, not a recovered original algorithm.
#[must_use]
pub const fn mix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::{RNG_ALGORITHM_VERSION, RngState, RngStreamKey, RngSubsystem};
    use crate::model::PlayerId;

    #[test]
    fn global_and_player_zero_streams_do_not_collide() {
        assert_ne!(
            RngStreamKey::global(RngSubsystem::Movement).selector(),
            RngStreamKey::for_player(PlayerId::from_raw(0), RngSubsystem::Movement).selector()
        );
    }

    #[test]
    fn identical_seed_and_stream_yield_identical_values() {
        let mut first = RngState::from_seed(1234);
        let mut second = RngState::from_seed(1234);
        let key = RngStreamKey::for_player(PlayerId::from_raw(1), RngSubsystem::Movement);

        let first_values: Vec<_> = (0..8).map(|_| first.draw_u64(key)).collect();
        let second_values: Vec<_> = (0..8).map(|_| second.draw_u64(key)).collect();

        assert_eq!(first_values, second_values);
        assert_eq!(first.algorithm_version(), RNG_ALGORITHM_VERSION);
    }

    #[test]
    fn streams_are_independent_and_ordered_by_stable_key() {
        let mut state = RngState::from_seed(99);
        let movement = RngStreamKey::for_player(PlayerId::from_raw(2), RngSubsystem::Movement);
        let combat = RngStreamKey::for_player(PlayerId::from_raw(2), RngSubsystem::Combat);
        let global = RngStreamKey::global(RngSubsystem::TurnSetup);

        let movement_first = state.draw_u64(movement);
        let combat_first = state.draw_u64(combat);
        let global_first = state.draw_u64(global);

        assert_ne!(movement_first, combat_first);
        assert_ne!(movement_first, global_first);
        assert_eq!(state.draws(movement), 1);
        assert_eq!(state.draws(combat), 1);
        assert_eq!(
            state.stream_keys(),
            vec![combat, movement, global]
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn drawing_from_one_stream_does_not_advance_another() {
        let mut state = RngState::from_seed(7);
        let movement = RngStreamKey::global(RngSubsystem::Movement);
        let research = RngStreamKey::global(RngSubsystem::Research);

        let before = state.draws(research);
        state.draw_u64(movement);

        assert_eq!(state.draws(research), before);
        assert_eq!(state.draws(movement), 1);
    }
}
