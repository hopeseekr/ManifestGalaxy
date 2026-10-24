//! Authoritative, deterministic game-state model and canonical state encoding.
//!
//! This module defines stable typed identifiers, integer-only positions, the
//! entity collections processed in stable key order, and the state envelope
//! shared by every later subsystem. It deliberately contains no economy,
//! combat or race formula; those arrive with their evidence-backed PRs.
//!
//! Display coordinates (floating point, viewport-relative) do not belong here.
//! Authoritative positions are integer [`GalacticPosition`] values. The desktop
//! layer may project them for rendering without writing floats back to state.

use std::collections::BTreeMap;
use std::fmt;

use crate::rng::RngState;
use crate::turn::{PhasePlan, PhasePlanError};

/// Version of the canonical state encoding produced by this module.
///
/// Version 3 adds generated planet details and each player's starting
/// technology.
pub const STATE_FORMAT_VERSION: u16 = 3;

/// Magic bytes at the start of every canonical state encoding.
pub const STATE_MAGIC: [u8; 4] = *b"STC4";

macro_rules! id_type {
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

        impl CanonicalEncode for $name {
            fn encode_canonical(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.0.to_le_bytes());
            }
        }

        impl $name {
            pub(crate) fn decode_canonical(
                reader: &mut CanonicalReader<'_>,
            ) -> Result<Self, StateDecodeError> {
                Ok(Self(reader.read_u64(concat!(stringify!($name), ".raw"))?))
            }
        }
    };
}

id_type!(PlayerId, "player");
id_type!(PlanetId, "planet");
id_type!(FleetId, "fleet");
id_type!(DesignId, "design");
id_type!(BatchId, "batch");

/// Authoritative integer position in the game universe.
///
/// The coordinate system, bounds and generation rules belong to PR6. This
/// contract only fixes the integer representation so replay never depends on a
/// floating-point projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GalacticPosition {
    /// Horizontal coordinate in authoritative units.
    pub x: i32,
    /// Vertical coordinate in authoritative units.
    pub y: i32,
}

impl GalacticPosition {
    /// Creates a position from explicit integer coordinates.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// How much of an entity is visible to a player in a filtered view.
///
/// This is a contract fixture for player-specific views, not a recovered
/// scanning or stealth rule. PR11 replaces the fixture inputs with scanning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityVisibility {
    /// Every player may see the entity.
    Public,
    /// Only the owning player may see the entity.
    OwnerOnly,
    /// No player view may contain the entity.
    Hidden,
}

impl EntityVisibility {
    const fn code(self) -> u8 {
        match self {
            Self::Public => 1,
            Self::OwnerOnly => 2,
            Self::Hidden => 3,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Public),
            2 => Some(Self::OwnerOnly),
            3 => Some(Self::Hidden),
            _ => None,
        }
    }

    /// Whether a player view should include the entity.
    #[must_use]
    pub const fn is_visible_to(self, owner: Option<PlayerId>, viewer: PlayerId) -> bool {
        match self {
            Self::Public => true,
            Self::OwnerOnly => matches!(owner, Some(owner) if owner.raw() == viewer.raw()),
            Self::Hidden => false,
        }
    }
}

/// Whether a player slot is controlled by the human or by the computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerRole {
    /// The single human custom-race player.
    Human,
    /// A computer opponent, such as Turindrones.
    Computer,
}

impl PlayerRole {
    const fn code(self) -> u8 {
        match self {
            Self::Human => 1,
            Self::Computer => 2,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Human),
            2 => Some(Self::Computer),
            _ => None,
        }
    }
}

/// One player slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    /// Stable identity.
    pub id: PlayerId,
    /// Display name. Race configuration arrives in PR5.
    pub name: String,
    /// Human or computer controller.
    pub role: PlayerRole,
    /// Whether this player has submitted orders for the current turn.
    pub ready: bool,
    /// Validated custom race for this player slot.
    ///
    /// PR5 supplies the human custom race; PR6 supplies the built-in
    /// Turindrones configuration for computer opponents. Unfinished setup slots
    /// use `None`, and loading validates any race that is present.
    pub race: Option<crate::race::RaceSpec>,
    /// Starting technology level in each of the six research fields.
    ///
    /// Index order is Energy, Weapons, Propulsion, Construction, Electronics and
    /// Biotechnology. PR6 derives these values from the recovered starting-tech
    /// rules; PR9 owns later research progress.
    pub starting_technology: [u8; 6],
}

/// Environmental values of one planet.
///
/// The original stores each value as a byte in `1..=100`, with `-1` marking an
/// immune race axis. A planet itself never has an immune axis, so this struct
/// uses unsigned bytes. Values are retained as generated; the reference
/// habitability reconstruction treats them as percentages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanetEnvironment {
    /// Gravity value.
    pub gravity: u8,
    /// Temperature value.
    pub temperature: u8,
    /// Radiation value.
    pub radiation: u8,
}

impl PlanetEnvironment {
    /// Creates an environment from its three values.
    #[must_use]
    pub const fn new(gravity: u8, temperature: u8, radiation: u8) -> Self {
        Self {
            gravity,
            temperature,
            radiation,
        }
    }

    /// Returns the three values in gravity, temperature, radiation order.
    #[must_use]
    pub const fn values(self) -> [u8; 3] {
        [self.gravity, self.temperature, self.radiation]
    }

    /// Returns the values as signed integers for the habitability model.
    #[must_use]
    pub const fn signed_values(self) -> [i32; 3] {
        [
            self.gravity as i32,
            self.temperature as i32,
            self.radiation as i32,
        ]
    }
}

/// One planet in the authoritative state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planet {
    /// Stable identity.
    pub id: PlanetId,
    /// Display name.
    pub name: String,
    /// Authoritative integer position.
    pub position: GalacticPosition,
    /// Current owner, if colonized.
    pub owner: Option<PlayerId>,
    /// Synthetic visibility fixture consumed by player views.
    pub visibility: EntityVisibility,
    /// Generated environmental values.
    pub environment: PlanetEnvironment,
    /// Mineral concentration for ironium, boranium and germanium, in `0..=200`.
    pub mineral_concentration: [u8; 3],
    /// Surface minerals for ironium, boranium and germanium.
    pub surface_minerals: [u32; 3],
    /// Colonist population in individuals.
    pub population: u32,
    /// Completed factories.
    pub factories: u32,
    /// Completed mines.
    pub mines: u32,
    /// Completed planetary defenses.
    pub defenses: u32,
    /// Whether this is a player's primary starting world.
    pub homeworld: bool,
    /// Whether a starbase is present at the start of the game.
    pub starbase: bool,
}

/// One ship design owned by a player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Design {
    /// Stable identity.
    pub id: DesignId,
    /// Owning player.
    pub owner: PlayerId,
    /// Display name.
    pub name: String,
    /// Synthetic visibility fixture consumed by player views.
    pub visibility: EntityVisibility,
}

/// Where a fleet is located.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleetLocation {
    /// In orbit around a planet.
    AtPlanet(PlanetId),
    /// In deep space at an integer position.
    InSpace(GalacticPosition),
}

/// One fleet in the authoritative state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fleet {
    /// Stable identity.
    pub id: FleetId,
    /// Owning player.
    pub owner: PlayerId,
    /// Display name.
    pub name: String,
    /// Referenced ship design.
    pub design: DesignId,
    /// Number of ships in the fleet.
    pub ships: u32,
    /// Current location.
    pub location: FleetLocation,
    /// Synthetic visibility fixture consumed by player views.
    pub visibility: EntityVisibility,
}

/// Monotonic allocator for stable entity identifiers.
///
/// Allocation never derives an identifier from map order, memory addresses or a
/// hash. The allocator values are part of serialized state so a reloaded game
/// continues without reusing identifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdAllocator {
    /// Next player identifier.
    pub next_player: u64,
    /// Next planet identifier.
    pub next_planet: u64,
    /// Next fleet identifier.
    pub next_fleet: u64,
    /// Next design identifier.
    pub next_design: u64,
}

impl IdAllocator {
    const fn new() -> Self {
        Self {
            next_player: 1,
            next_planet: 1,
            next_fleet: 1,
            next_design: 1,
        }
    }
}

/// Failure to allocate a stable identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdAllocatorError {
    /// The integer identifier space for that entity kind is exhausted.
    Exhausted,
}

impl fmt::Display for IdAllocatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("stable identifier space exhausted")
    }
}

impl std::error::Error for IdAllocatorError {}

/// Authoritative game state.
///
/// Mutate through [`crate::turn::GameSession`] when processing orders so a
/// failed batch cannot leave a partial turn behind. Direct field access exists
/// for state construction, fixtures and read-only views.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    /// Core contract version this state was created for.
    pub contract_version: u32,
    /// Calendar year shown to players. All games start in 2400 (PR6 setup).
    pub year: u32,
    /// Number of completed turn generations. New games start at turn `0`.
    pub turn: u32,
    /// Explicit phase order recorded in state.
    pub phase_plan: PhasePlan,
    /// Versioned random state.
    pub rng: RngState,
    /// Stable identifier allocation cursors.
    pub next_ids: IdAllocator,
    /// Players ordered by stable identifier.
    pub players: BTreeMap<PlayerId, Player>,
    /// Designs ordered by stable identifier.
    pub designs: BTreeMap<DesignId, Design>,
    /// Planets ordered by stable identifier.
    pub planets: BTreeMap<PlanetId, Planet>,
    /// Fleets ordered by stable identifier.
    pub fleets: BTreeMap<FleetId, Fleet>,
    /// Last accepted order batch, used to reject duplicate or stale batches.
    pub last_applied_batch: Option<BatchId>,
    /// Next event sequence number.
    pub next_event_sequence: u64,
}

impl GameState {
    /// Creates an empty contract state with the designed phase plan.
    ///
    /// `year` and `seed` are explicit because universe and new-game setup are
    /// owned by PR6. This constructor invents no starting inventory.
    #[must_use]
    pub fn new(year: u32, seed: u64) -> Self {
        Self {
            contract_version: crate::CORE_API_VERSION,
            year,
            turn: 0,
            phase_plan: PhasePlan::designed_v1(),
            rng: RngState::from_seed(seed),
            next_ids: IdAllocator::new(),
            players: BTreeMap::new(),
            designs: BTreeMap::new(),
            planets: BTreeMap::new(),
            fleets: BTreeMap::new(),
            last_applied_batch: None,
            next_event_sequence: 0,
        }
    }

    /// Replaces the phase order while preserving unrelated state.
    ///
    /// This is the supported way to adopt a future evidence-backed order. The
    /// replacement is versioned and validated before it is stored.
    ///
    /// # Errors
    ///
    /// Returns [`PhasePlanError`] when the plan is empty, duplicated, malformed
    /// or records an unsupported version.
    pub fn replace_phase_plan(&mut self, plan: PhasePlan) -> Result<(), PhasePlanError> {
        plan.validate()?;
        self.phase_plan = plan;
        Ok(())
    }

    /// Allocates the next stable player identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdAllocatorError::Exhausted`] at the integer limit.
    pub fn allocate_player_id(&mut self) -> Result<PlayerId, IdAllocatorError> {
        allocate(&mut self.next_ids.next_player).map(PlayerId::from_raw)
    }

    /// Allocates the next stable planet identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdAllocatorError::Exhausted`] at the integer limit.
    pub fn allocate_planet_id(&mut self) -> Result<PlanetId, IdAllocatorError> {
        allocate(&mut self.next_ids.next_planet).map(PlanetId::from_raw)
    }

    /// Allocates the next stable fleet identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdAllocatorError::Exhausted`] at the integer limit.
    pub fn allocate_fleet_id(&mut self) -> Result<FleetId, IdAllocatorError> {
        allocate(&mut self.next_ids.next_fleet).map(FleetId::from_raw)
    }

    /// Allocates the next stable design identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdAllocatorError::Exhausted`] at the integer limit.
    pub fn allocate_design_id(&mut self) -> Result<DesignId, IdAllocatorError> {
        allocate(&mut self.next_ids.next_design).map(DesignId::from_raw)
    }

    /// Checks contract version, allocator cursors and entity references.
    ///
    /// # Errors
    ///
    /// Returns [`StateValidationError`] on the first violation found in stable
    /// key order.
    pub fn validate_integrity(&self) -> Result<(), StateValidationError> {
        if self.contract_version != crate::CORE_API_VERSION {
            return Err(StateValidationError::UnsupportedContractVersion(
                self.contract_version,
            ));
        }
        self.phase_plan
            .validate()
            .map_err(StateValidationError::PhasePlan)?;

        if self.next_event_sequence == u64::MAX {
            return Err(StateValidationError::EventSequenceExhausted);
        }

        check_allocator(
            "player",
            self.next_ids.next_player,
            self.players.keys().map(|id| id.raw()),
        )?;
        check_allocator(
            "design",
            self.next_ids.next_design,
            self.designs.keys().map(|id| id.raw()),
        )?;
        check_allocator(
            "planet",
            self.next_ids.next_planet,
            self.planets.keys().map(|id| id.raw()),
        )?;
        check_allocator(
            "fleet",
            self.next_ids.next_fleet,
            self.fleets.keys().map(|id| id.raw()),
        )?;

        // Each entity's own identifier must match its map key; encoding uses
        // the entity field while decoding re-keys by it, so a mismatch would
        // produce bytes that decode into a different state.
        for (key, player) in &self.players {
            if player.id != *key {
                return Err(StateValidationError::EntityIdMismatch {
                    kind: "player",
                    key: key.raw(),
                    entity: player.id.raw(),
                });
            }
            if let Some(race) = &player.race
                && let Some(error) = crate::race::validate_race_fields(race).into_iter().next()
            {
                return Err(StateValidationError::InvalidRace {
                    player: player.id,
                    field: error.field_key(),
                });
            }
            if let Some((field, value)) = player
                .starting_technology
                .iter()
                .copied()
                .enumerate()
                .find(|(_, value)| *value > 26)
            {
                return Err(StateValidationError::InvalidStartingTechnology {
                    player: player.id,
                    field,
                    value,
                });
            }
        }
        for (key, design) in &self.designs {
            if design.id != *key {
                return Err(StateValidationError::EntityIdMismatch {
                    kind: "design",
                    key: key.raw(),
                    entity: design.id.raw(),
                });
            }
        }
        for (key, planet) in &self.planets {
            if planet.id != *key {
                return Err(StateValidationError::EntityIdMismatch {
                    kind: "planet",
                    key: key.raw(),
                    entity: planet.id.raw(),
                });
            }
            if let Some(value) = planet
                .environment
                .values()
                .into_iter()
                .find(|value| *value > 100)
            {
                return Err(StateValidationError::InvalidPlanetEnvironment {
                    planet: planet.id,
                    value,
                });
            }
            if let Some(value) = planet
                .mineral_concentration
                .into_iter()
                .find(|value| *value > 200)
            {
                return Err(StateValidationError::InvalidMineralConcentration {
                    planet: planet.id,
                    value,
                });
            }
            if planet.homeworld && planet.owner.is_none() {
                return Err(StateValidationError::HomeworldWithoutOwner { planet: planet.id });
            }
            if planet.homeworld && !planet.starbase {
                return Err(StateValidationError::HomeworldWithoutStarbase { planet: planet.id });
            }
        }
        for (key, fleet) in &self.fleets {
            if fleet.id != *key {
                return Err(StateValidationError::EntityIdMismatch {
                    kind: "fleet",
                    key: key.raw(),
                    entity: fleet.id.raw(),
                });
            }
        }

        for design in self.designs.values() {
            if !self.players.contains_key(&design.owner) {
                return Err(StateValidationError::UnknownPlayer {
                    referenced_by: "design",
                    id: design.id.raw(),
                    player: design.owner,
                });
            }
        }
        for planet in self.planets.values() {
            if let Some(owner) = planet.owner
                && !self.players.contains_key(&owner)
            {
                return Err(StateValidationError::UnknownPlayer {
                    referenced_by: "planet",
                    id: planet.id.raw(),
                    player: owner,
                });
            }
        }
        for fleet in self.fleets.values() {
            if !self.players.contains_key(&fleet.owner) {
                return Err(StateValidationError::UnknownPlayer {
                    referenced_by: "fleet",
                    id: fleet.id.raw(),
                    player: fleet.owner,
                });
            }
            if !self.designs.contains_key(&fleet.design) {
                return Err(StateValidationError::UnknownDesign {
                    fleet: fleet.id,
                    design: fleet.design,
                });
            }
            if let FleetLocation::AtPlanet(planet) = fleet.location
                && !self.planets.contains_key(&planet)
            {
                return Err(StateValidationError::UnknownPlanet {
                    fleet: fleet.id,
                    planet,
                });
            }
        }

        Ok(())
    }

    /// Serializes the complete authoritative state to canonical bytes.
    ///
    /// The encoding is little-endian, length-prefixed, ordered by stable
    /// identifiers, and carries the state format and contract versions. It is
    /// a determinism and replay contract, not the PR16 save-file envelope.
    #[must_use]
    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&STATE_MAGIC);
        out.extend_from_slice(&STATE_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&self.contract_version.to_le_bytes());
        out.extend_from_slice(&self.year.to_le_bytes());
        out.extend_from_slice(&self.turn.to_le_bytes());
        self.phase_plan.encode_canonical(&mut out);
        self.rng.encode_canonical(&mut out);
        self.next_ids.encode_canonical(&mut out);
        match self.last_applied_batch {
            Some(batch) => {
                out.push(1);
                batch.encode_canonical(&mut out);
            }
            None => out.push(0),
        }
        out.extend_from_slice(&self.next_event_sequence.to_le_bytes());

        encode_len(&mut out, self.players.len());
        for player in self.players.values() {
            player.encode_canonical(&mut out);
        }
        encode_len(&mut out, self.designs.len());
        for design in self.designs.values() {
            design.encode_canonical(&mut out);
        }
        encode_len(&mut out, self.planets.len());
        for planet in self.planets.values() {
            planet.encode_canonical(&mut out);
        }
        encode_len(&mut out, self.fleets.len());
        for fleet in self.fleets.values() {
            fleet.encode_canonical(&mut out);
        }
        out
    }

    /// Decodes and validates a canonical state encoding.
    ///
    /// # Errors
    ///
    /// Returns [`StateDecodeError`] for bad magic, unsupported versions,
    /// truncated or trailing bytes, duplicate identifiers, invalid tags,
    /// invalid UTF-8 or broken entity references.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self, StateDecodeError> {
        let mut reader = CanonicalReader::new(bytes);
        let magic = reader.read_exact("magic", STATE_MAGIC.len())?;
        if magic != STATE_MAGIC {
            return Err(reader.error(StateDecodeErrorKind::BadMagic));
        }
        let format_version = reader.read_u16("format_version")?;
        if format_version != STATE_FORMAT_VERSION {
            return Err(reader.error(StateDecodeErrorKind::UnsupportedFormatVersion(
                format_version,
            )));
        }
        let contract_version = reader.read_u32("contract_version")?;
        if contract_version != crate::CORE_API_VERSION {
            return Err(
                reader.error(StateDecodeErrorKind::UnsupportedContractVersion(
                    contract_version,
                )),
            );
        }
        let year = reader.read_u32("year")?;
        let turn = reader.read_u32("turn")?;
        let phase_plan = PhasePlan::decode_canonical(&mut reader)?;
        let rng = RngState::decode_canonical(&mut reader)?;
        let next_ids = IdAllocator::decode_canonical(&mut reader)?;
        let last_applied_batch = if reader.read_bool("last_applied_batch.present")? {
            Some(BatchId::decode_canonical(&mut reader)?)
        } else {
            None
        };
        let next_event_sequence = reader.read_u64("next_event_sequence")?;

        let player_count = reader.read_len("players")?;
        let mut players = BTreeMap::new();
        for _ in 0..player_count {
            let player = Player::decode_canonical(&mut reader)?;
            if players.insert(player.id, player).is_some() {
                return Err(reader.error(StateDecodeErrorKind::DuplicateId("player")));
            }
        }
        let design_count = reader.read_len("designs")?;
        let mut designs = BTreeMap::new();
        for _ in 0..design_count {
            let design = Design::decode_canonical(&mut reader)?;
            if designs.insert(design.id, design).is_some() {
                return Err(reader.error(StateDecodeErrorKind::DuplicateId("design")));
            }
        }
        let planet_count = reader.read_len("planets")?;
        let mut planets = BTreeMap::new();
        for _ in 0..planet_count {
            let planet = Planet::decode_canonical(&mut reader)?;
            if planets.insert(planet.id, planet).is_some() {
                return Err(reader.error(StateDecodeErrorKind::DuplicateId("planet")));
            }
        }
        let fleet_count = reader.read_len("fleets")?;
        let mut fleets = BTreeMap::new();
        for _ in 0..fleet_count {
            let fleet = Fleet::decode_canonical(&mut reader)?;
            if fleets.insert(fleet.id, fleet).is_some() {
                return Err(reader.error(StateDecodeErrorKind::DuplicateId("fleet")));
            }
        }

        if !reader.is_finished() {
            return Err(reader.error(StateDecodeErrorKind::TrailingBytes));
        }

        let state = Self {
            contract_version,
            year,
            turn,
            phase_plan,
            rng,
            next_ids,
            players,
            designs,
            planets,
            fleets,
            last_applied_batch,
            next_event_sequence,
        };
        state
            .validate_integrity()
            .map_err(|error| reader.error(StateDecodeErrorKind::InvalidState(error.to_string())))?;
        Ok(state)
    }
}

fn allocate(next: &mut u64) -> Result<u64, IdAllocatorError> {
    let value = *next;
    *next = next.checked_add(1).ok_or(IdAllocatorError::Exhausted)?;
    Ok(value)
}

fn check_allocator(
    kind: &'static str,
    next: u64,
    ids: impl Iterator<Item = u64>,
) -> Result<(), StateValidationError> {
    if let Some(max) = ids.max()
        && next <= max
    {
        return Err(StateValidationError::AllocatorNotAhead { kind, next, max });
    }
    Ok(())
}

/// A state-integrity violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateValidationError {
    /// State was built for a different public core contract.
    UnsupportedContractVersion(u32),
    /// The recorded phase plan is invalid.
    PhasePlan(PhasePlanError),
    /// An allocator cursor would permit identifier reuse.
    AllocatorNotAhead {
        /// Entity kind.
        kind: &'static str,
        /// Recorded next identifier.
        next: u64,
        /// Largest identifier already present.
        max: u64,
    },
    /// A reference names a player that is not present.
    UnknownPlayer {
        /// Entity kind containing the reference.
        referenced_by: &'static str,
        /// Identifier of the containing entity.
        id: u64,
        /// Missing player.
        player: PlayerId,
    },
    /// A fleet references a design that is not present.
    UnknownDesign {
        /// Referring fleet.
        fleet: FleetId,
        /// Missing design.
        design: DesignId,
    },
    /// A stored entity's `id` field does not match the map key it is stored
    /// under, so canonical encoding and decoding would disagree.
    EntityIdMismatch {
        /// Entity kind.
        kind: &'static str,
        /// Map key the entity is stored under.
        key: u64,
        /// Identifier recorded inside the entity.
        entity: u64,
    },
    /// A fleet references a planet that is not present.
    UnknownPlanet {
        /// Referring fleet.
        fleet: FleetId,
        /// Missing planet.
        planet: PlanetId,
    },
    /// The next event sequence has exhausted its range, so the next event
    /// would wrap to zero and break strictly increasing ordering.
    EventSequenceExhausted,
    /// A stored custom race has a field outside its recovered range.
    InvalidRace {
        /// Player carrying the race.
        player: PlayerId,
        /// First offending field key.
        field: &'static str,
    },
    /// A stored starting technology level exceeds the recovered maximum.
    InvalidStartingTechnology {
        /// Player carrying the value.
        player: PlayerId,
        /// Research field index.
        field: usize,
        /// Offending value.
        value: u8,
    },
    /// A planet environment value exceeds 100.
    InvalidPlanetEnvironment {
        /// Planet carrying the value.
        planet: PlanetId,
        /// Offending value.
        value: u8,
    },
    /// A mineral concentration exceeds 200.
    InvalidMineralConcentration {
        /// Planet carrying the value.
        planet: PlanetId,
        /// Offending value.
        value: u8,
    },
    /// A homeworld has no owner.
    HomeworldWithoutOwner {
        /// Offending planet.
        planet: PlanetId,
    },
    /// A homeworld has no starting starbase.
    HomeworldWithoutStarbase {
        /// Offending planet.
        planet: PlanetId,
    },
}

impl fmt::Display for StateValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedContractVersion(version) => {
                write!(formatter, "unsupported core contract version {version}")
            }
            Self::PhasePlan(error) => write!(formatter, "invalid phase plan: {error}"),
            Self::AllocatorNotAhead { kind, next, max } => write!(
                formatter,
                "{kind} allocator next id {next} is not ahead of existing id {max}"
            ),
            Self::UnknownPlayer {
                referenced_by,
                id,
                player,
            } => write!(
                formatter,
                "{referenced_by} {id} references missing player {}",
                player.raw()
            ),
            Self::UnknownDesign { fleet, design } => write!(
                formatter,
                "fleet {} references missing design {}",
                fleet.raw(),
                design.raw()
            ),
            Self::UnknownPlanet { fleet, planet } => write!(
                formatter,
                "fleet {} references missing planet {}",
                fleet.raw(),
                planet.raw()
            ),
            Self::EntityIdMismatch { kind, key, entity } => write!(
                formatter,
                "{kind} entity id {entity} does not match its map key {key}"
            ),
            Self::EventSequenceExhausted => {
                write!(formatter, "next event sequence exhausted its range")
            }
            Self::InvalidRace { player, field } => write!(
                formatter,
                "player {} has an invalid race field {field}",
                player.raw()
            ),
            Self::InvalidStartingTechnology {
                player,
                field,
                value,
            } => write!(
                formatter,
                "player {} has starting technology {value} in field {field}",
                player.raw()
            ),
            Self::InvalidPlanetEnvironment { planet, value } => write!(
                formatter,
                "planet {} has environment value {value} above 100",
                planet.raw()
            ),
            Self::InvalidMineralConcentration { planet, value } => write!(
                formatter,
                "planet {} has mineral concentration {value} above 200",
                planet.raw()
            ),
            Self::HomeworldWithoutOwner { planet } => {
                write!(formatter, "homeworld {} has no owner", planet.raw())
            }
            Self::HomeworldWithoutStarbase { planet } => {
                write!(formatter, "homeworld {} has no starbase", planet.raw())
            }
        }
    }
}

impl std::error::Error for StateValidationError {}

/// Failure to decode canonical state bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDecodeError {
    offset: usize,
    kind: StateDecodeErrorKind,
}

impl StateDecodeError {
    /// Byte offset at which decoding failed.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Typed failure reason.
    #[must_use]
    pub const fn kind(&self) -> &StateDecodeErrorKind {
        &self.kind
    }
}

impl fmt::Display for StateDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "state decode failed at byte {}: ", self.offset)?;
        match &self.kind {
            StateDecodeErrorKind::UnexpectedEof { field } => {
                write!(formatter, "unexpected end of input reading {field}")
            }
            StateDecodeErrorKind::BadMagic => formatter.write_str("bad state magic"),
            StateDecodeErrorKind::UnsupportedFormatVersion(version) => {
                write!(formatter, "unsupported state format version {version}")
            }
            StateDecodeErrorKind::UnsupportedContractVersion(version) => {
                write!(formatter, "unsupported core contract version {version}")
            }
            StateDecodeErrorKind::UnsupportedRngVersion(version) => {
                write!(formatter, "unsupported RNG version {version}")
            }
            StateDecodeErrorKind::UnsupportedPhasePlanVersion(version) => {
                write!(formatter, "unsupported phase plan version {version}")
            }
            StateDecodeErrorKind::InvalidTag { field, value } => {
                write!(formatter, "invalid tag {value} for {field}")
            }
            StateDecodeErrorKind::InvalidUtf8 { field } => {
                write!(formatter, "invalid UTF-8 in {field}")
            }
            StateDecodeErrorKind::InvalidLength { field, length } => {
                write!(formatter, "implausible length {length} for {field}")
            }
            StateDecodeErrorKind::DuplicateId(kind) => {
                write!(formatter, "duplicate {kind} identifier")
            }
            StateDecodeErrorKind::DuplicateValue(field) => {
                write!(formatter, "duplicate value in {field}")
            }
            StateDecodeErrorKind::TrailingBytes => {
                formatter.write_str("trailing bytes after state")
            }
            StateDecodeErrorKind::InvalidState(message) => {
                write!(formatter, "invalid decoded state: {message}")
            }
        }
    }
}

impl std::error::Error for StateDecodeError {}

/// Typed reason for a canonical decoding failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateDecodeErrorKind {
    /// Input ended before a field was complete.
    UnexpectedEof {
        /// Field being read.
        field: &'static str,
    },
    /// The state magic did not match [`STATE_MAGIC`].
    BadMagic,
    /// The state format is not supported by this build.
    UnsupportedFormatVersion(u16),
    /// The state was created for another core contract.
    UnsupportedContractVersion(u32),
    /// The RNG algorithm version is not supported.
    UnsupportedRngVersion(u16),
    /// The phase-plan version is not supported.
    UnsupportedPhasePlanVersion(u16),
    /// A discriminant byte has no defined meaning.
    InvalidTag {
        /// Field being read.
        field: &'static str,
        /// Unknown value.
        value: u8,
    },
    /// A length-prefixed string was not UTF-8.
    InvalidUtf8 {
        /// Field being read.
        field: &'static str,
    },
    /// A length prefix exceeded the remaining input.
    InvalidLength {
        /// Field being read.
        field: &'static str,
        /// Declared length.
        length: u32,
    },
    /// An identifier occurred more than once in a collection.
    DuplicateId(&'static str),
    /// A map or set contained the same key twice.
    DuplicateValue(&'static str),
    /// Input continued after a complete state.
    TrailingBytes,
    /// Decoded fields violated authoritative state invariants.
    InvalidState(String),
}

pub(crate) trait CanonicalEncode {
    fn encode_canonical(&self, out: &mut Vec<u8>);
}

pub(crate) fn encode_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("collection length exceeds canonical encoding limit");
    out.extend_from_slice(&len.to_le_bytes());
}

pub(crate) struct CanonicalReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> CanonicalReader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(crate) const fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub(crate) fn error(&self, kind: StateDecodeErrorKind) -> StateDecodeError {
        StateDecodeError {
            offset: self.offset,
            kind,
        }
    }

    pub(crate) fn read_exact(
        &mut self,
        field: &'static str,
        length: usize,
    ) -> Result<&'a [u8], StateDecodeError> {
        let end = self.offset.checked_add(length).ok_or_else(|| {
            self.error(StateDecodeErrorKind::InvalidLength {
                field,
                length: u32::try_from(length).unwrap_or(u32::MAX),
            })
        })?;
        if end > self.bytes.len() {
            return Err(self.error(StateDecodeErrorKind::UnexpectedEof { field }));
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    pub(crate) fn read_u8(&mut self, field: &'static str) -> Result<u8, StateDecodeError> {
        Ok(self.read_exact(field, 1)?[0])
    }

    pub(crate) fn read_bool(&mut self, field: &'static str) -> Result<bool, StateDecodeError> {
        match self.read_u8(field)? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(self.error(StateDecodeErrorKind::InvalidTag { field, value })),
        }
    }

    pub(crate) fn read_u16(&mut self, field: &'static str) -> Result<u16, StateDecodeError> {
        let bytes = self.read_exact(field, 2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub(crate) fn read_u32(&mut self, field: &'static str) -> Result<u32, StateDecodeError> {
        let bytes = self.read_exact(field, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(crate) fn read_i32(&mut self, field: &'static str) -> Result<i32, StateDecodeError> {
        let bytes = self.read_exact(field, 4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(crate) fn read_u64(&mut self, field: &'static str) -> Result<u64, StateDecodeError> {
        let bytes = self.read_exact(field, 8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub(crate) fn read_len(&mut self, field: &'static str) -> Result<u32, StateDecodeError> {
        let length = self.read_u32(field)?;
        if length as usize > self.remaining() {
            return Err(self.error(StateDecodeErrorKind::InvalidLength { field, length }));
        }
        Ok(length)
    }

    pub(crate) fn read_string(&mut self, field: &'static str) -> Result<String, StateDecodeError> {
        let length = self.read_len(field)? as usize;
        let bytes = self.read_exact(field, length)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| self.error(StateDecodeErrorKind::InvalidUtf8 { field }))
    }
}

pub(crate) fn encode_string(out: &mut Vec<u8>, value: &str) {
    encode_len(out, value.len());
    out.extend_from_slice(value.as_bytes());
}

impl CanonicalEncode for Player {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        self.id.encode_canonical(out);
        encode_string(out, &self.name);
        out.push(self.role.code());
        out.push(u8::from(self.ready));
        match &self.race {
            Some(race) => {
                out.push(1);
                race.encode_canonical(out);
            }
            None => out.push(0),
        }
        out.extend_from_slice(&self.starting_technology);
    }
}

impl Player {
    fn decode_canonical(reader: &mut CanonicalReader<'_>) -> Result<Self, StateDecodeError> {
        let id = PlayerId::decode_canonical(reader)?;
        let name = reader.read_string("player.name")?;
        let role_code = reader.read_u8("player.role")?;
        let role = PlayerRole::from_code(role_code).ok_or_else(|| {
            reader.error(StateDecodeErrorKind::InvalidTag {
                field: "player.role",
                value: role_code,
            })
        })?;
        let ready = reader.read_bool("player.ready")?;
        let race = if reader.read_bool("player.race.present")? {
            Some(crate::race::RaceSpec::decode_canonical(reader)?)
        } else {
            None
        };
        let technology = reader.read_exact("player.starting_technology", 6)?;
        let mut starting_technology = [0_u8; 6];
        starting_technology.copy_from_slice(technology);
        Ok(Self {
            id,
            name,
            role,
            ready,
            race,
            starting_technology,
        })
    }
}

impl CanonicalEncode for Design {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        self.id.encode_canonical(out);
        self.owner.encode_canonical(out);
        encode_string(out, &self.name);
        out.push(self.visibility.code());
    }
}

impl Design {
    fn decode_canonical(reader: &mut CanonicalReader<'_>) -> Result<Self, StateDecodeError> {
        let id = DesignId::decode_canonical(reader)?;
        let owner = PlayerId::decode_canonical(reader)?;
        let name = reader.read_string("design.name")?;
        let visibility_code = reader.read_u8("design.visibility")?;
        let visibility = EntityVisibility::from_code(visibility_code).ok_or_else(|| {
            reader.error(StateDecodeErrorKind::InvalidTag {
                field: "design.visibility",
                value: visibility_code,
            })
        })?;
        Ok(Self {
            id,
            owner,
            name,
            visibility,
        })
    }
}

impl CanonicalEncode for GalacticPosition {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.x.to_le_bytes());
        out.extend_from_slice(&self.y.to_le_bytes());
    }
}

impl GalacticPosition {
    fn decode_canonical(reader: &mut CanonicalReader<'_>) -> Result<Self, StateDecodeError> {
        Ok(Self {
            x: reader.read_i32("position.x")?,
            y: reader.read_i32("position.y")?,
        })
    }
}

impl CanonicalEncode for Planet {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        self.id.encode_canonical(out);
        encode_string(out, &self.name);
        self.position.encode_canonical(out);
        match self.owner {
            Some(owner) => {
                out.push(1);
                owner.encode_canonical(out);
            }
            None => out.push(0),
        }
        out.push(self.visibility.code());
        out.extend_from_slice(&self.environment.values());
        out.extend_from_slice(&self.mineral_concentration);
        for value in self.surface_minerals {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&self.population.to_le_bytes());
        out.extend_from_slice(&self.factories.to_le_bytes());
        out.extend_from_slice(&self.mines.to_le_bytes());
        out.extend_from_slice(&self.defenses.to_le_bytes());
        out.push(u8::from(self.homeworld));
        out.push(u8::from(self.starbase));
    }
}

impl Planet {
    fn decode_canonical(reader: &mut CanonicalReader<'_>) -> Result<Self, StateDecodeError> {
        let id = PlanetId::decode_canonical(reader)?;
        let name = reader.read_string("planet.name")?;
        let position = GalacticPosition::decode_canonical(reader)?;
        let owner = if reader.read_bool("planet.owner.present")? {
            Some(PlayerId::decode_canonical(reader)?)
        } else {
            None
        };
        let visibility_code = reader.read_u8("planet.visibility")?;
        let visibility = EntityVisibility::from_code(visibility_code).ok_or_else(|| {
            reader.error(StateDecodeErrorKind::InvalidTag {
                field: "planet.visibility",
                value: visibility_code,
            })
        })?;
        let environment = reader.read_exact("planet.environment", 3)?;
        let environment = PlanetEnvironment::new(environment[0], environment[1], environment[2]);
        let concentration = reader.read_exact("planet.mineral_concentration", 3)?;
        let mut mineral_concentration = [0_u8; 3];
        mineral_concentration.copy_from_slice(concentration);

        let mut surface_minerals = [0_u32; 3];
        for value in &mut surface_minerals {
            *value = reader.read_u32("planet.surface_minerals")?;
        }

        Ok(Self {
            id,
            name,
            position,
            owner,
            visibility,
            environment,
            mineral_concentration,
            surface_minerals,
            population: reader.read_u32("planet.population")?,
            factories: reader.read_u32("planet.factories")?,
            mines: reader.read_u32("planet.mines")?,
            defenses: reader.read_u32("planet.defenses")?,
            homeworld: reader.read_bool("planet.homeworld")?,
            starbase: reader.read_bool("planet.starbase")?,
        })
    }
}

impl CanonicalEncode for FleetLocation {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        match self {
            Self::AtPlanet(planet) => {
                out.push(1);
                planet.encode_canonical(out);
            }
            Self::InSpace(position) => {
                out.push(2);
                position.encode_canonical(out);
            }
        }
    }
}

impl FleetLocation {
    fn decode_canonical(reader: &mut CanonicalReader<'_>) -> Result<Self, StateDecodeError> {
        let tag = reader.read_u8("fleet.location")?;
        match tag {
            1 => Ok(Self::AtPlanet(PlanetId::decode_canonical(reader)?)),
            2 => Ok(Self::InSpace(GalacticPosition::decode_canonical(reader)?)),
            value => Err(reader.error(StateDecodeErrorKind::InvalidTag {
                field: "fleet.location",
                value,
            })),
        }
    }
}

impl CanonicalEncode for Fleet {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        self.id.encode_canonical(out);
        self.owner.encode_canonical(out);
        encode_string(out, &self.name);
        self.design.encode_canonical(out);
        out.extend_from_slice(&self.ships.to_le_bytes());
        self.location.encode_canonical(out);
        out.push(self.visibility.code());
    }
}

impl Fleet {
    fn decode_canonical(reader: &mut CanonicalReader<'_>) -> Result<Self, StateDecodeError> {
        let id = FleetId::decode_canonical(reader)?;
        let owner = PlayerId::decode_canonical(reader)?;
        let name = reader.read_string("fleet.name")?;
        let design = DesignId::decode_canonical(reader)?;
        let ships = reader.read_u32("fleet.ships")?;
        let location = FleetLocation::decode_canonical(reader)?;
        let visibility_code = reader.read_u8("fleet.visibility")?;
        let visibility = EntityVisibility::from_code(visibility_code).ok_or_else(|| {
            reader.error(StateDecodeErrorKind::InvalidTag {
                field: "fleet.visibility",
                value: visibility_code,
            })
        })?;
        Ok(Self {
            id,
            owner,
            name,
            design,
            ships,
            location,
            visibility,
        })
    }
}

impl CanonicalEncode for IdAllocator {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.next_player.to_le_bytes());
        out.extend_from_slice(&self.next_planet.to_le_bytes());
        out.extend_from_slice(&self.next_fleet.to_le_bytes());
        out.extend_from_slice(&self.next_design.to_le_bytes());
    }
}

impl IdAllocator {
    fn decode_canonical(reader: &mut CanonicalReader<'_>) -> Result<Self, StateDecodeError> {
        Ok(Self {
            next_player: reader.read_u64("next_ids.player")?,
            next_planet: reader.read_u64("next_ids.planet")?,
            next_fleet: reader.read_u64("next_ids.fleet")?,
            next_design: reader.read_u64("next_ids.design")?,
        })
    }
}
