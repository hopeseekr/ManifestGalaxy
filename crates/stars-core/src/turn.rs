//! Turn pipeline, deterministic phase dispatch, events and atomic commits.
//!
//! The global cross-phase order of the original game is not recovered
//! (`RULE-TURN-ORDER-001`, `GAP-TURN-001`). The pipeline below is an explicit
//! designed choice recorded in serialized state. It follows the known local
//! constraint that minesweeping happens before movement, but it must not be
//! described as verified original behaviour.
//!
//! Only administrative order intake and the terminal snapshot phase are
//! implemented in PR4. Every other phase is an explicit
//! [`PhaseCapability::NotImplemented`] no-op and emits an event saying so.

use std::collections::BTreeSet;
use std::fmt;

use crate::model::{
    BatchId, CanonicalReader, FleetId, GameState, PlayerId, StateDecodeError, StateDecodeErrorKind,
    StateValidationError,
};
use crate::orders::{Command, CommandAction, CommandId, OrderBatch, OrderError};
use crate::view::{PlayerView, ViewError};

/// Version of the designed phase ordering and capability table.
pub const PHASE_PLAN_VERSION: u16 = 1;

/// One phase category in the turn pipeline.
///
/// The categories come from the PR2 turn-order investigation. Their order here
/// is a designed remake order, not a recovered original order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Computer-opponent planning (reserved for PR14).
    AiPlanning,
    /// Human order intake and administrative effects.
    OrderIntake,
    /// Production-queue progress (reserved for PR8).
    Production,
    /// Research progress (reserved for PR9).
    Research,
    /// Construction completion (reserved for PR8/PR9).
    Construction,
    /// Terraforming and environmental change (reserved for PR8/PR13).
    Terraforming,
    /// Population growth and deaths (reserved for PR8).
    PopulationGrowth,
    /// Mining and depletion (reserved for PR8).
    Mining,
    /// Mine laying, decay and sweeping (reserved for PR10/PR12).
    Minesweeping,
    /// Fleet movement (reserved for PR10).
    Movement,
    /// Waypoint tasks (reserved for PR10).
    WaypointTasks,
    /// Packet travel and impact (reserved for PR10/PR12).
    PacketTravel,
    /// Scanning and visibility updates (reserved for PR11).
    Scanning,
    /// Fleet, starbase and ground combat (reserved for PR12).
    Combat,
    /// Scrapping and recycling credit (reserved for PR9/PR10).
    Salvage,
    /// Random events and artifact effects (reserved for a later PR).
    RandomEvents,
    /// Victory evaluation (reserved for PR15).
    Victory,
    /// Snapshot, event filtering and player-view update.
    Snapshot,
}

impl Phase {
    /// Every phase in the designed v1 order.
    pub const ALL_V1: [Self; 18] = [
        Self::AiPlanning,
        Self::OrderIntake,
        Self::Production,
        Self::Research,
        Self::Construction,
        Self::Terraforming,
        Self::PopulationGrowth,
        Self::Mining,
        Self::Minesweeping,
        Self::Movement,
        Self::WaypointTasks,
        Self::PacketTravel,
        Self::Scanning,
        Self::Combat,
        Self::Salvage,
        Self::RandomEvents,
        Self::Victory,
        Self::Snapshot,
    ];

    /// Stable encoded value. Do not renumber.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::AiPlanning => 1,
            Self::OrderIntake => 2,
            Self::Production => 3,
            Self::Research => 4,
            Self::Construction => 5,
            Self::Terraforming => 6,
            Self::PopulationGrowth => 7,
            Self::Mining => 8,
            Self::Minesweeping => 9,
            Self::Movement => 10,
            Self::WaypointTasks => 11,
            Self::PacketTravel => 12,
            Self::Scanning => 13,
            Self::Combat => 14,
            Self::Salvage => 15,
            Self::RandomEvents => 16,
            Self::Victory => 17,
            Self::Snapshot => 18,
        }
    }

    /// Decodes a stable value written by [`Self::code`].
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::AiPlanning),
            2 => Some(Self::OrderIntake),
            3 => Some(Self::Production),
            4 => Some(Self::Research),
            5 => Some(Self::Construction),
            6 => Some(Self::Terraforming),
            7 => Some(Self::PopulationGrowth),
            8 => Some(Self::Mining),
            9 => Some(Self::Minesweeping),
            10 => Some(Self::Movement),
            11 => Some(Self::WaypointTasks),
            12 => Some(Self::PacketTravel),
            13 => Some(Self::Scanning),
            14 => Some(Self::Combat),
            15 => Some(Self::Salvage),
            16 => Some(Self::RandomEvents),
            17 => Some(Self::Victory),
            18 => Some(Self::Snapshot),
            _ => None,
        }
    }

    /// Human-readable contract name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AiPlanning => "ai-planning",
            Self::OrderIntake => "order-intake",
            Self::Production => "production",
            Self::Research => "research",
            Self::Construction => "construction",
            Self::Terraforming => "terraforming",
            Self::PopulationGrowth => "population-growth",
            Self::Mining => "mining",
            Self::Minesweeping => "minesweeping",
            Self::Movement => "movement",
            Self::WaypointTasks => "waypoint-tasks",
            Self::PacketTravel => "packet-travel",
            Self::Scanning => "scanning",
            Self::Combat => "combat",
            Self::Salvage => "salvage",
            Self::RandomEvents => "random-events",
            Self::Victory => "victory",
            Self::Snapshot => "snapshot",
        }
    }
}

/// What a phase is allowed to do in this build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseCapability {
    /// The phase performs its PR4 contract work.
    Implemented,
    /// The phase is a deliberate no-op until its owning PR supplies rules.
    NotImplemented,
}

impl PhaseCapability {
    /// Whether this phase changes gameplay state in the current build.
    #[must_use]
    pub const fn is_implemented(self) -> bool {
        matches!(self, Self::Implemented)
    }
}

/// Provenance of a recorded phase order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhasePlanSource {
    /// Chosen by the remake as a deterministic contract because the original
    /// cross-phase order is unknown.
    DesignedChoice,
}

impl PhasePlanSource {
    const fn code(self) -> u8 {
        match self {
            Self::DesignedChoice => 1,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::DesignedChoice),
            _ => None,
        }
    }

    /// Human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DesignedChoice => "designed",
        }
    }
}

/// Versioned, serialized phase order and its provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhasePlan {
    version: u16,
    source: PhasePlanSource,
    phases: Vec<Phase>,
}

impl PhasePlan {
    /// Creates the designed v1 order.
    #[must_use]
    pub fn designed_v1() -> Self {
        Self {
            version: PHASE_PLAN_VERSION,
            source: PhasePlanSource::DesignedChoice,
            phases: Phase::ALL_V1.to_vec(),
        }
    }

    /// Builds a plan from an explicit ordered phase list.
    ///
    /// # Errors
    ///
    /// Returns [`PhasePlanError`] when the list is empty, duplicated, too long
    /// for stable encoding, or contains an invalid phase code. All known
    /// phases are currently valid; validation is still explicit so a future
    /// restored plan is checked.
    pub fn from_phases(phases: Vec<Phase>) -> Result<Self, PhasePlanError> {
        let plan = Self {
            version: PHASE_PLAN_VERSION,
            source: PhasePlanSource::DesignedChoice,
            phases,
        };
        plan.validate()?;
        Ok(plan)
    }

    /// Version of this plan.
    #[must_use]
    pub const fn version(&self) -> u16 {
        self.version
    }

    /// Why this order was chosen.
    #[must_use]
    pub const fn source(&self) -> PhasePlanSource {
        self.source
    }

    /// Ordered phases.
    #[must_use]
    pub fn phases(&self) -> &[Phase] {
        &self.phases
    }

    /// Capability of a phase in the current build.
    #[must_use]
    pub const fn capability(&self, phase: Phase) -> PhaseCapability {
        match phase {
            Phase::OrderIntake | Phase::Snapshot => PhaseCapability::Implemented,
            _ => PhaseCapability::NotImplemented,
        }
    }

    /// Validates version, count, uniqueness and phase codes.
    ///
    /// # Errors
    ///
    /// Returns [`PhasePlanError`] on the first violation.
    pub fn validate(&self) -> Result<(), PhasePlanError> {
        if self.version != PHASE_PLAN_VERSION {
            return Err(PhasePlanError::UnsupportedVersion(self.version));
        }
        if self.phases.is_empty() {
            return Err(PhasePlanError::Empty);
        }
        if self.phases.len() > 256 {
            return Err(PhasePlanError::TooLong(self.phases.len()));
        }
        let mut seen = BTreeSet::new();
        for phase in &self.phases {
            if !seen.insert(*phase) {
                return Err(PhasePlanError::Duplicate(*phase));
            }
        }
        if !seen.contains(&Phase::OrderIntake) {
            return Err(PhasePlanError::MissingPhase(Phase::OrderIntake));
        }
        if !seen.contains(&Phase::Snapshot) {
            return Err(PhasePlanError::MissingPhase(Phase::Snapshot));
        }
        if self.phases.last() != Some(&Phase::Snapshot) {
            return Err(PhasePlanError::SnapshotNotLast);
        }
        Ok(())
    }

    pub(crate) fn encode_canonical(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.version.to_le_bytes());
        out.push(self.source.code());
        crate::model::encode_len(out, self.phases.len());
        for phase in &self.phases {
            out.push(phase.code());
        }
    }

    pub(crate) fn decode_canonical(
        reader: &mut CanonicalReader<'_>,
    ) -> Result<Self, StateDecodeError> {
        let version = reader.read_u16("phase_plan.version")?;
        if version != PHASE_PLAN_VERSION {
            return Err(reader.error(StateDecodeErrorKind::UnsupportedPhasePlanVersion(version)));
        }
        let source_code = reader.read_u8("phase_plan.source")?;
        let source = PhasePlanSource::from_code(source_code).ok_or_else(|| {
            reader.error(StateDecodeErrorKind::InvalidTag {
                field: "phase_plan.source",
                value: source_code,
            })
        })?;
        let count = reader.read_len("phase_plan.phases")?;
        let mut phases = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let code = reader.read_u8("phase_plan.phase")?;
            let phase = Phase::from_code(code).ok_or_else(|| {
                reader.error(StateDecodeErrorKind::InvalidTag {
                    field: "phase_plan.phase",
                    value: code,
                })
            })?;
            phases.push(phase);
        }
        let plan = Self {
            version,
            source,
            phases,
        };
        plan.validate()
            .map_err(|error| reader.error(StateDecodeErrorKind::InvalidState(error.to_string())))?;
        Ok(plan)
    }
}

impl Default for PhasePlan {
    fn default() -> Self {
        Self::designed_v1()
    }
}

/// Invalid phase plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhasePlanError {
    /// The version is not understood by this build.
    UnsupportedVersion(u16),
    /// The plan contains no phases.
    Empty,
    /// The plan exceeds the stable encoding limit.
    TooLong(usize),
    /// A phase occurs more than once.
    Duplicate(Phase),
    /// A required contract phase is absent.
    MissingPhase(Phase),
    /// The terminal snapshot phase is not last.
    SnapshotNotLast,
}

impl fmt::Display for PhasePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported phase plan version {version}")
            }
            Self::Empty => formatter.write_str("phase plan is empty"),
            Self::TooLong(length) => write!(formatter, "phase plan has too many phases: {length}"),
            Self::Duplicate(phase) => write!(formatter, "phase {} occurs twice", phase.name()),
            Self::MissingPhase(phase) => {
                write!(formatter, "required phase {} is absent", phase.name())
            }
            Self::SnapshotNotLast => formatter.write_str("snapshot phase is not last"),
        }
    }
}

impl std::error::Error for PhasePlanError {}

/// Who may observe an event in a player-filtered view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventVisibility {
    /// Visible to every player.
    Public,
    /// Visible only to the listed players.
    Players(BTreeSet<PlayerId>),
    /// Not exposed in any player view.
    Hidden,
}

impl EventVisibility {
    /// Whether an event may appear in the given player's view.
    #[must_use]
    pub fn is_visible_to(&self, player: PlayerId) -> bool {
        match self {
            Self::Public => true,
            Self::Players(players) => players.contains(&player),
            Self::Hidden => false,
        }
    }

    fn for_player(player: PlayerId) -> Self {
        let mut players = BTreeSet::new();
        players.insert(player);
        Self::Players(players)
    }
}

/// Typed event payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// A new turn generation began.
    TurnStarted {
        /// Year before generation.
        year: u32,
    },
    /// One administrative command was applied.
    CommandApplied {
        /// Command identifier.
        command: CommandId,
        /// Submitting player.
        player: PlayerId,
    },
    /// A player's readiness changed.
    PlayerReadinessChanged {
        /// Player.
        player: PlayerId,
        /// New readiness state.
        ready: bool,
    },
    /// A fleet was renamed.
    FleetRenamed {
        /// Fleet.
        fleet: FleetId,
        /// New name.
        name: String,
    },
    /// An implemented phase completed.
    PhaseCompleted {
        /// Phase that ran.
        phase: Phase,
    },
    /// A phase was deliberately skipped because its rules are not implemented.
    PhaseNotImplemented {
        /// Skipped phase.
        phase: Phase,
    },
    /// The turn was committed atomically.
    TurnCommitted {
        /// Accepted batch.
        batch: BatchId,
        /// New year.
        year: u32,
    },
}

/// One ordered game event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameEvent {
    /// Strictly increasing sequence number within a game.
    pub sequence: u64,
    /// Turn on which the event was produced.
    pub turn: u32,
    /// Player visibility.
    pub visibility: EventVisibility,
    /// Typed payload.
    pub kind: EventKind,
}

impl GameEvent {
    /// Applies the event visibility filter.
    #[must_use]
    pub fn is_visible_to(&self, player: PlayerId) -> bool {
        self.visibility.is_visible_to(player)
    }
}

/// Reporting callback invoked once per phase before the phase runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseProgress {
    /// Phase about to run.
    pub phase: Phase,
    /// Zero-based index in the recorded plan.
    pub index: u32,
    /// Total phases in the recorded plan.
    pub total: u32,
    /// Whether the phase is implemented in this build.
    pub capability: PhaseCapability,
}

/// Result of one atomically committed turn generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnOutcome {
    /// Accepted batch.
    pub batch: BatchId,
    /// Submitting player.
    pub player: PlayerId,
    /// New authoritative turn number.
    pub turn: u32,
    /// New calendar year.
    pub year: u32,
    /// Ordered events produced by this generation.
    pub events: Vec<GameEvent>,
    /// Complete authoritative snapshot after the commit.
    pub state: GameState,
    /// Filtered view for the submitting player.
    pub view: PlayerView,
}

/// Authoritative session wrapper that applies batches atomically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameSession {
    state: GameState,
    last_events: Vec<GameEvent>,
}

impl GameSession {
    /// Creates a session around a valid state.
    ///
    /// # Errors
    ///
    /// Returns [`StateValidationError`] if the state has broken references,
    /// identifier reuse or an invalid phase plan.
    pub fn new(state: GameState) -> Result<Self, StateValidationError> {
        state.validate_integrity()?;
        Ok(Self {
            state,
            last_events: Vec::new(),
        })
    }

    /// Current authoritative state.
    #[must_use]
    pub const fn state(&self) -> &GameState {
        &self.state
    }

    /// Events from the last committed generation.
    #[must_use]
    pub fn last_events(&self) -> &[GameEvent] {
        &self.last_events
    }

    /// Builds a player-filtered view of the current state and last events.
    ///
    /// # Errors
    ///
    /// Returns [`ViewError`] if the player does not exist or the state contains
    /// a dangling reference. Sessions validate state on construction, so the
    /// latter indicates later direct mutation.
    pub fn view_for(&self, player: PlayerId) -> Result<PlayerView, ViewError> {
        PlayerView::build(&self.state, player, &self.last_events)
    }

    /// Validates and atomically commits one order batch.
    ///
    /// # Errors
    ///
    /// Returns [`OrderError`] without changing the session when the batch is
    /// stale, duplicated, unauthorized or references missing entities.
    pub fn commit(&mut self, batch: &OrderBatch) -> Result<TurnOutcome, OrderError> {
        self.commit_observed(batch, |_| {})
    }

    /// Commits one batch while reporting phase progress.
    ///
    /// The callback is observational only: it receives copied values and cannot
    /// change state or RNG consumption. Progress ordering is therefore part of
    /// the deterministic contract.
    ///
    /// # Errors
    ///
    /// Returns [`OrderError`] without changing the session on validation or
    /// arithmetic failure.
    pub fn commit_observed<F>(
        &mut self,
        batch: &OrderBatch,
        mut observe: F,
    ) -> Result<TurnOutcome, OrderError>
    where
        F: FnMut(PhaseProgress),
    {
        let resolved = validate_batch(&self.state, batch)?;
        let mut candidate = self.state.clone();
        let mut events = Vec::new();
        let phases = candidate.phase_plan.phases().to_vec();
        let total = u32::try_from(phases.len()).map_err(|_| OrderError::TurnLimitReached)?;

        for (index, phase) in phases.iter().copied().enumerate() {
            let capability = candidate.phase_plan.capability(phase);
            observe(PhaseProgress {
                phase,
                index: u32::try_from(index).map_err(|_| OrderError::TurnLimitReached)?,
                total,
                capability,
            });

            match phase {
                Phase::OrderIntake => {
                    let start_year = candidate.year;
                    push_event(
                        &mut candidate,
                        &mut events,
                        EventVisibility::Public,
                        EventKind::TurnStarted { year: start_year },
                    );
                    for (command_id, action) in &resolved {
                        apply_command(&mut candidate, batch.player, *command_id, action);
                        push_event(
                            &mut candidate,
                            &mut events,
                            EventVisibility::for_player(batch.player),
                            EventKind::CommandApplied {
                                command: *command_id,
                                player: batch.player,
                            },
                        );
                        match action {
                            ResolvedCommand::SetReady(ready) => push_event(
                                &mut candidate,
                                &mut events,
                                EventVisibility::for_player(batch.player),
                                EventKind::PlayerReadinessChanged {
                                    player: batch.player,
                                    ready: *ready,
                                },
                            ),
                            ResolvedCommand::RenameFleet { fleet, name } => push_event(
                                &mut candidate,
                                &mut events,
                                EventVisibility::for_player(batch.player),
                                EventKind::FleetRenamed {
                                    fleet: *fleet,
                                    name: name.clone(),
                                },
                            ),
                        }
                    }
                    push_event(
                        &mut candidate,
                        &mut events,
                        EventVisibility::Public,
                        EventKind::PhaseCompleted { phase },
                    );
                }
                Phase::Snapshot => {
                    let next_turn = candidate
                        .turn
                        .checked_add(1)
                        .ok_or(OrderError::TurnLimitReached)?;
                    let next_year = candidate
                        .year
                        .checked_add(1)
                        .ok_or(OrderError::TurnLimitReached)?;
                    candidate.turn = next_turn;
                    candidate.year = next_year;
                    push_event(
                        &mut candidate,
                        &mut events,
                        EventVisibility::Public,
                        EventKind::PhaseCompleted { phase },
                    );
                    push_event(
                        &mut candidate,
                        &mut events,
                        EventVisibility::Public,
                        EventKind::TurnCommitted {
                            batch: batch.id,
                            year: next_year,
                        },
                    );
                }
                _ => {
                    push_event(
                        &mut candidate,
                        &mut events,
                        EventVisibility::Public,
                        EventKind::PhaseNotImplemented { phase },
                    );
                }
            }
        }

        candidate.last_applied_batch = Some(batch.id);
        candidate
            .validate_integrity()
            .map_err(|error| OrderError::InvalidState(error.to_string()))?;
        let view = PlayerView::build(&candidate, batch.player, &events)
            .map_err(|error| OrderError::InvalidState(error.to_string()))?;

        self.state = candidate.clone();
        self.last_events = events.clone();

        Ok(TurnOutcome {
            batch: batch.id,
            player: batch.player,
            turn: candidate.turn,
            year: candidate.year,
            events,
            state: candidate,
            view,
        })
    }
}

enum ResolvedCommand {
    SetReady(bool),
    RenameFleet { fleet: FleetId, name: String },
}

fn validate_batch(
    state: &GameState,
    batch: &OrderBatch,
) -> Result<Vec<(CommandId, ResolvedCommand)>, OrderError> {
    if state
        .last_applied_batch
        .is_some_and(|last| batch.id <= last)
    {
        return Err(OrderError::DuplicateOrStaleBatch {
            batch: batch.id,
            last_applied: state.last_applied_batch,
        });
    }
    if batch.expected_turn != state.turn {
        return Err(OrderError::StaleTurn {
            expected: batch.expected_turn,
            actual: state.turn,
        });
    }
    if !state.players.contains_key(&batch.player) {
        return Err(OrderError::UnknownPlayer(batch.player));
    }

    let mut seen: BTreeSet<CommandId> = BTreeSet::new();
    let mut resolved = Vec::with_capacity(batch.commands.len());
    for Command { id, action } in &batch.commands {
        if !seen.insert(*id) {
            return Err(OrderError::DuplicateCommand(*id));
        }
        match action {
            CommandAction::SetReady { ready } => {
                resolved.push((*id, ResolvedCommand::SetReady(*ready)));
            }
            CommandAction::RenameFleet { fleet, name } => {
                let existing = state
                    .fleets
                    .get(fleet)
                    .ok_or(OrderError::UnknownFleet(*fleet))?;
                if existing.owner != batch.player {
                    return Err(OrderError::UnauthorizedFleet {
                        fleet: *fleet,
                        owner: existing.owner,
                        player: batch.player,
                    });
                }
                resolved.push((
                    *id,
                    ResolvedCommand::RenameFleet {
                        fleet: *fleet,
                        name: name.clone(),
                    },
                ));
            }
        }
    }
    Ok(resolved)
}

fn apply_command(
    state: &mut GameState,
    player: PlayerId,
    _command: CommandId,
    action: &ResolvedCommand,
) {
    match action {
        ResolvedCommand::SetReady(ready) => {
            if let Some(record) = state.players.get_mut(&player) {
                record.ready = *ready;
            }
        }
        ResolvedCommand::RenameFleet { fleet, name } => {
            if let Some(record) = state.fleets.get_mut(fleet) {
                record.name.clone_from(name);
            }
        }
    }
}

fn push_event(
    state: &mut GameState,
    events: &mut Vec<GameEvent>,
    visibility: EventVisibility,
    kind: EventKind,
) {
    events.push(GameEvent {
        sequence: state.next_event_sequence,
        turn: state.turn,
        visibility,
        kind,
    });
    state.next_event_sequence = state.next_event_sequence.wrapping_add(1);
}

/// Returns the visible event subset in original order.
#[must_use]
pub fn filter_events(events: &[GameEvent], player: PlayerId) -> Vec<GameEvent> {
    events
        .iter()
        .filter(|event| event.is_visible_to(player))
        .cloned()
        .collect()
}
