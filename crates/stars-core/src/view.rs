//! Player-filtered projections of authoritative state and events.
//!
//! These views are the only state shape a player or computer opponent should
//! receive. Entity visibility is a synthetic contract fixture for PR4; PR11
//! replaces the input flags with scanning-derived visibility. Events use the
//! same filter. Hidden entities and hidden events never appear.

use std::fmt;

use crate::model::{
    Design, DesignId, EntityVisibility, FleetId, FleetLocation, GalacticPosition, GameState,
    Planet, PlanetEnvironment, PlanetId, PlayerId, PlayerRole,
};
use crate::race::{LesserTrait, PrimaryTrait};
use crate::turn::GameEvent;

/// One visible design in a player view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignView {
    /// Stable identity.
    pub id: DesignId,
    /// Owning player.
    pub owner: PlayerId,
    /// Display name.
    pub name: String,
}

/// Player-visible summary of the viewer's own race.
///
/// The password and other builder-editing fields are deliberately excluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceView {
    /// Singular race name.
    pub name: String,
    /// Plural race name.
    pub plural_name: String,
    /// Selected primary racial trait.
    pub primary: PrimaryTrait,
    /// Selected lesser racial traits in stable trait order.
    pub lesser: Vec<LesserTrait>,
    /// Starting technology in the six research fields.
    pub starting_technology: [u8; 6],
}

/// One visible planet in a player view.
///
/// All detail fields describe the same authoritative planet that produced the
/// visibility decision. When a later scanning PR hides a fact, it must omit the
/// whole planet from the view rather than relying on the desktop to redact it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanetView {
    /// Stable identity.
    pub id: PlanetId,
    /// Display name.
    pub name: String,
    /// Authoritative integer position.
    pub position: GalacticPosition,
    /// Current owner, if visible.
    pub owner: Option<PlayerId>,
    /// Generated environmental values.
    pub environment: PlanetEnvironment,
    /// Mineral concentration for ironium, boranium and germanium.
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
    /// Whether a starbase is present.
    pub starbase: bool,
}

/// Visible location of a fleet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewLocation {
    /// In orbit around a visible planet.
    AtPlanet {
        /// Referenced planet.
        planet: PlanetId,
        /// Integer position copied from the planet for presentation.
        position: GalacticPosition,
    },
    /// In orbit around a planet whose identity and position are not visible
    /// to this player.
    UnknownOrbit,
    /// In deep space.
    InSpace(GalacticPosition),
}

/// One visible fleet in a player view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FleetView {
    /// Stable identity.
    pub id: FleetId,
    /// Owning player.
    pub owner: PlayerId,
    /// Display name.
    pub name: String,
    /// Referenced design, if that design is visible to this player.
    pub design: Option<DesignId>,
    /// Number of ships.
    pub ships: u32,
    /// Visible location.
    pub location: ViewLocation,
}

/// Filtered state and events for one player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerView {
    /// Viewing player.
    pub player: PlayerId,
    /// Viewing player's display name.
    pub player_name: String,
    /// Viewing player's controller role.
    pub role: PlayerRole,
    /// The viewer's own race summary, if the slot has a race.
    pub race: Option<RaceView>,
    /// Whether the viewer has submitted orders.
    pub ready: bool,
    /// Current year.
    pub year: u32,
    /// Current turn.
    pub turn: u32,
    /// Visible designs ordered by stable identifier.
    pub designs: Vec<DesignView>,
    /// Visible planets ordered by stable identifier.
    pub planets: Vec<PlanetView>,
    /// Visible fleets ordered by stable identifier.
    pub fleets: Vec<FleetView>,
    /// Visible events in original sequence order.
    pub events: Vec<GameEvent>,
}

impl PlayerView {
    /// Builds a filtered view from authoritative state and events.
    ///
    /// # Errors
    ///
    /// Returns [`ViewError`] for an unknown viewer or a dangling reference in
    /// directly mutated state.
    pub fn build(
        state: &GameState,
        player: PlayerId,
        events: &[GameEvent],
    ) -> Result<Self, ViewError> {
        let viewer = state
            .players
            .get(&player)
            .ok_or(ViewError::UnknownPlayer(player))?;

        let mut designs = Vec::new();
        for design in state.designs.values() {
            require_player(state, "design", design.id.raw(), design.owner)?;
            if is_visible(design.visibility, Some(design.owner), player) {
                designs.push(design_view(design));
            }
        }

        let mut planets = Vec::new();
        for planet in state.planets.values() {
            if let Some(owner) = planet.owner {
                require_player(state, "planet", planet.id.raw(), owner)?;
            }
            if is_visible(planet.visibility, planet.owner, player) {
                planets.push(planet_view(planet));
            }
        }

        let mut fleets = Vec::new();
        for fleet in state.fleets.values() {
            require_player(state, "fleet", fleet.id.raw(), fleet.owner)?;
            if !state.designs.contains_key(&fleet.design) {
                return Err(ViewError::DanglingDesign {
                    fleet: fleet.id,
                    design: fleet.design,
                });
            }
            let location = match fleet.location {
                FleetLocation::AtPlanet(planet_id) => {
                    let planet =
                        state
                            .planets
                            .get(&planet_id)
                            .ok_or(ViewError::DanglingPlanet {
                                fleet: fleet.id,
                                planet: planet_id,
                            })?;
                    // A visible fleet must not disclose the identity or
                    // position of a planet this player cannot see.
                    if is_visible(planet.visibility, planet.owner, player) {
                        ViewLocation::AtPlanet {
                            planet: planet_id,
                            position: planet.position,
                        }
                    } else {
                        ViewLocation::UnknownOrbit
                    }
                }
                FleetLocation::InSpace(position) => ViewLocation::InSpace(position),
            };
            let fleet_visible = is_visible(fleet.visibility, Some(fleet.owner), player);
            if fleet_visible {
                // A visible fleet must not disclose the identity of a design
                // this player cannot see; the design list omits it too.
                let design = state
                    .designs
                    .get(&fleet.design)
                    .is_some_and(|design| is_visible(design.visibility, Some(design.owner), player))
                    .then_some(fleet.design);
                fleets.push(FleetView {
                    id: fleet.id,
                    owner: fleet.owner,
                    name: fleet.name.clone(),
                    design,
                    ships: fleet.ships,
                    location,
                });
            }
        }

        let events = events
            .iter()
            .filter(|event| event.is_visible_to(player))
            .cloned()
            .collect();

        Ok(Self {
            player,
            player_name: viewer.name.clone(),
            role: viewer.role,
            race: viewer
                .race
                .as_ref()
                .map(|race| race_view(race, viewer.starting_technology)),
            ready: viewer.ready,
            year: state.year,
            turn: state.turn,
            designs,
            planets,
            fleets,
            events,
        })
    }
}

fn is_visible(visibility: EntityVisibility, owner: Option<PlayerId>, viewer: PlayerId) -> bool {
    visibility.is_visible_to(owner, viewer)
}

fn require_player(
    state: &GameState,
    referenced_by: &'static str,
    id: u64,
    player: PlayerId,
) -> Result<(), ViewError> {
    if state.players.contains_key(&player) {
        Ok(())
    } else {
        Err(ViewError::DanglingPlayer {
            referenced_by,
            id,
            player,
        })
    }
}

fn design_view(design: &Design) -> DesignView {
    DesignView {
        id: design.id,
        owner: design.owner,
        name: design.name.clone(),
    }
}

fn race_view(race: &crate::race::RaceSpec, starting_technology: [u8; 6]) -> RaceView {
    RaceView {
        name: race.name.clone(),
        plural_name: race.plural_name.clone(),
        primary: race.primary,
        lesser: race.lesser.iter().copied().collect(),
        starting_technology,
    }
}

fn planet_view(planet: &Planet) -> PlanetView {
    PlanetView {
        id: planet.id,
        name: planet.name.clone(),
        position: planet.position,
        owner: planet.owner,
        environment: planet.environment,
        mineral_concentration: planet.mineral_concentration,
        surface_minerals: planet.surface_minerals,
        population: planet.population,
        factories: planet.factories,
        mines: planet.mines,
        defenses: planet.defenses,
        homeworld: planet.homeworld,
        starbase: planet.starbase,
    }
}

/// Failure to build a player view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewError {
    /// The viewer is not present in the game.
    UnknownPlayer(PlayerId),
    /// An entity references a missing player.
    DanglingPlayer {
        /// Entity kind containing the reference.
        referenced_by: &'static str,
        /// Containing entity identifier.
        id: u64,
        /// Missing player.
        player: PlayerId,
    },
    /// A fleet references a missing design.
    DanglingDesign {
        /// Referring fleet.
        fleet: FleetId,
        /// Missing design.
        design: DesignId,
    },
    /// A fleet references a missing planet.
    DanglingPlanet {
        /// Referring fleet.
        fleet: FleetId,
        /// Missing planet.
        planet: PlanetId,
    },
}

impl fmt::Display for ViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlayer(player) => {
                write!(formatter, "player {} does not exist", player.raw())
            }
            Self::DanglingPlayer {
                referenced_by,
                id,
                player,
            } => write!(
                formatter,
                "{referenced_by} {id} references missing player {}",
                player.raw()
            ),
            Self::DanglingDesign { fleet, design } => write!(
                formatter,
                "fleet {} references missing design {}",
                fleet.raw(),
                design.raw()
            ),
            Self::DanglingPlanet { fleet, planet } => write!(
                formatter,
                "fleet {} references missing planet {}",
                fleet.raw(),
                planet.raw()
            ),
        }
    }
}

impl std::error::Error for ViewError {}
