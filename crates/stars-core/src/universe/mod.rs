//! Universe generation and new-game settings.
//!
//! All gameplay values live in core. The desktop package owns the form and the
//! worker thread; it never recomputes planet counts, positions, starting
//! inventory or the Turindrones race.

pub mod generate;
pub mod settings;
pub mod turindrones;

pub use generate::{
    GENERATION_STEPS, GeneratedUniverse, UniverseGenerationError, UniverseGenerationStage,
    UniverseProgress, generate_universe, generate_universe_with_progress, starting_technology,
};
pub use settings::{
    Density, Difficulty, GAME_NAME_MAX_BYTES, GalaxySize, GameOptions, MAX_OPPONENTS, MAX_PLAYERS,
    PlayerPositions, UniverseSettings, UniverseSettingsError, VictoryCondition,
    VictoryConditionKind, VictorySettings,
};
pub use turindrones::{TURINDRONES_NAME, turindrones_race};
