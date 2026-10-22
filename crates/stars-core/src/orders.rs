//! Player orders and typed batch validation errors.
//!
//! PR4 supports only administrative order effects that need no unrecovered
//! gameplay formula: readiness submission and fleet renaming. Movement,
//! production, research, combat, colonization, diplomacy and AI planning keep
//! their later PR ownership. Unknown gameplay commands are therefore absent
//! rather than silently accepted as no-ops.

use std::fmt;

use crate::model::{BatchId, FleetId, PlayerId};

macro_rules! command_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
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

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}-{}", $description, self.0)
            }
        }
    };
}

command_id!(CommandId, "command");

/// One ordered command inside a player's batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// Caller-supplied identifier, unique within the batch.
    pub id: CommandId,
    /// Administrative effect requested by the player.
    pub action: CommandAction,
}

impl Command {
    /// Creates a command.
    #[must_use]
    pub const fn new(id: CommandId, action: CommandAction) -> Self {
        Self { id, action }
    }
}

/// Supported administrative command effects.
///
/// These are contract operations, not recovered gameplay rules. They mutate
/// only descriptive or submission state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandAction {
    /// Marks the submitting player as ready or not ready for turn generation.
    SetReady {
        /// Requested readiness state.
        ready: bool,
    },
    /// Renames one fleet owned by the submitting player.
    RenameFleet {
        /// Fleet to rename.
        fleet: FleetId,
        /// New display name.
        name: String,
    },
}

/// An ordered batch of commands from one player for one expected turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderBatch {
    /// Monotonic batch identifier chosen by the caller. Reusing or going
    /// backwards is rejected as a duplicate or stale batch.
    pub id: BatchId,
    /// Submitting player.
    pub player: PlayerId,
    /// Turn number the caller believes is current.
    pub expected_turn: u32,
    /// Commands in submission order.
    pub commands: Vec<Command>,
}

impl OrderBatch {
    /// Creates an empty batch. An empty batch is a valid end-turn submission.
    #[must_use]
    pub const fn new(id: BatchId, player: PlayerId, expected_turn: u32) -> Self {
        Self {
            id,
            player,
            expected_turn,
            commands: Vec::new(),
        }
    }

    /// Appends a command in submission order.
    #[must_use]
    pub fn with_command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }
}

/// Typed rejection of an order batch.
///
/// Every variant leaves authoritative state unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderError {
    /// The batch identifier is not strictly newer than the last accepted batch.
    DuplicateOrStaleBatch {
        /// Rejected batch.
        batch: BatchId,
        /// Last accepted batch, if any.
        last_applied: Option<BatchId>,
    },
    /// The batch was prepared for a different turn than the authoritative one.
    StaleTurn {
        /// Turn recorded by the caller.
        expected: u32,
        /// Current authoritative turn.
        actual: u32,
    },
    /// The submitting player does not exist in this game.
    UnknownPlayer(PlayerId),
    /// Two commands in the batch used the same identifier.
    DuplicateCommand(CommandId),
    /// A command references a fleet that does not exist.
    UnknownFleet(FleetId),
    /// A command references a fleet owned by another player.
    UnauthorizedFleet {
        /// Referenced fleet.
        fleet: FleetId,
        /// Actual owner.
        owner: PlayerId,
        /// Submitting player.
        player: PlayerId,
    },
    /// The current turn number cannot advance without integer overflow.
    TurnLimitReached,
    /// The authoritative state itself is invalid.
    InvalidState(String),
}

impl fmt::Display for OrderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateOrStaleBatch {
                batch,
                last_applied,
            } => match last_applied {
                Some(last) => write!(
                    formatter,
                    "batch {} is not newer than accepted batch {}",
                    batch.raw(),
                    last.raw()
                ),
                None => write!(formatter, "batch {} is not a fresh batch", batch.raw()),
            },
            Self::StaleTurn { expected, actual } => write!(
                formatter,
                "batch targets turn {expected}, but the current turn is {actual}"
            ),
            Self::UnknownPlayer(player) => {
                write!(formatter, "player {} does not exist", player.raw())
            }
            Self::DuplicateCommand(command) => {
                write!(formatter, "command {} occurs more than once", command.raw())
            }
            Self::UnknownFleet(fleet) => write!(formatter, "fleet {} does not exist", fleet.raw()),
            Self::UnauthorizedFleet {
                fleet,
                owner,
                player,
            } => write!(
                formatter,
                "player {} cannot command fleet {} owned by player {}",
                player.raw(),
                fleet.raw(),
                owner.raw()
            ),
            Self::TurnLimitReached => formatter.write_str("turn number limit reached"),
            Self::InvalidState(message) => write!(formatter, "invalid state: {message}"),
        }
    }
}

impl std::error::Error for OrderError {}
