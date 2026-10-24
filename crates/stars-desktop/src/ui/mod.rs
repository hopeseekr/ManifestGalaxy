//! Desktop user-interface modules.

pub mod inspector;
pub mod map;
pub mod new_game;
pub mod orders;
pub mod race_builder;
pub mod reports;

pub use map::{MapState, MapTransform, Selection, WorldBounds};
pub use new_game::{
    NewGameActivity, NewGameError, NewGameState, UniverseJob, UniverseWorkerError,
    UniverseWorkerHandle, UniverseWorkerMessage,
};
pub use orders::{DraftStatus, OrderDraft};
pub use race_builder::{RaceBuilderState, RacePage};
pub use reports::{OwnerFilter, ReportState, ReportTable};
