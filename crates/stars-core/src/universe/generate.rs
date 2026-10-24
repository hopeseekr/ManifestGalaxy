//! Deterministic universe and starting-world generation.
//!
//! The planet count, coordinate span, minimum separation, clumping algorithm,
//! environment rolls, mineral rolls, homeworld placement and starting inventory
//! below follow `GenerateWorld()` and `InitNewGamePlr()` in the decompiled
//! Stars! 2.60j RC3 source (`stars-decompile/create.c`). They are statically
//! derived from a later build than the supplied `stars26i.exe`; no original
//! execution was available. Where the later source and the 1998 help disagree,
//! the difference is recorded in `docs/rules/universe-generation.md` instead of
//! being silently smoothed over.
//!
//! The random-number mapping is a designed remake choice. The original RNG
//! algorithm and seed handling are unknown (`GAP-RNG-001`), so this module
//! consumes a separate deterministic universe stream derived from the explicit
//! seed. The resulting [`GameState`] records the same seed for later turns.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    EntityVisibility, GalacticPosition, GameState, Planet, PlanetEnvironment, PlanetId, Player,
    PlayerId, PlayerRole,
};
use crate::race::{
    LesserTrait, PrimaryTrait, RaceSpec, RaceValidationError, ReferenceHabitability, ResearchCost,
    evaluate_race, reference_desirability, validate_race,
};
use crate::rng::mix64;
use crate::universe::settings::{
    Difficulty, GAME_NAME_MAX_BYTES, MAX_OPPONENTS, MAX_PLAYERS, UniverseSettings,
    UniverseSettingsError,
};
use crate::universe::turindrones::turindrones_race;

/// Number of progress checkpoints emitted by generation.
pub const GENERATION_STEPS: u32 = 6;

/// A milestone in universe generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UniverseGenerationStage {
    /// Settings and the human race are validated.
    Validating,
    /// Planet counts and coordinates are generated.
    Positions,
    /// Environments and minerals are generated.
    Environments,
    /// Player homeworlds are selected.
    Homeworlds,
    /// Players, starting technology and second worlds are initialized.
    Players,
    /// The completed state and fixture summary are ready.
    Complete,
}

impl UniverseGenerationStage {
    /// UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Validating => "Validating settings",
            Self::Positions => "Generating star positions",
            Self::Environments => "Generating environments and minerals",
            Self::Homeworlds => "Placing home worlds",
            Self::Players => "Initializing players",
            Self::Complete => "Universe complete",
        }
    }
}

/// One progress checkpoint from universe generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UniverseProgress {
    /// Current stage.
    pub stage: UniverseGenerationStage,
    /// Completed checkpoints.
    pub completed_steps: u32,
    /// Total checkpoints.
    pub total_steps: u32,
}

impl UniverseProgress {
    /// Completion fraction in `0.0..=1.0`.
    #[must_use]
    pub fn fraction(self) -> f32 {
        if self.total_steps == 0 {
            1.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            let fraction = f32::from(u16::try_from(self.completed_steps).unwrap_or(u16::MAX))
                / f32::from(u16::try_from(self.total_steps).unwrap_or(u16::MAX));
            fraction.min(1.0)
        }
    }
}

/// A generated universe and the player identities that own it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedUniverse {
    /// Settings that produced this universe.
    pub settings: UniverseSettings,
    /// Authoritative starting state.
    pub state: GameState,
    /// The single human player.
    pub human: PlayerId,
    /// Computer opponents in stable player-identifier order.
    pub opponents: Vec<PlayerId>,
    /// Primary homeworld for each player.
    pub homeworlds: BTreeMap<PlayerId, PlanetId>,
}

impl GeneratedUniverse {
    /// Number of generated planets.
    #[must_use]
    pub fn planet_count(&self) -> usize {
        self.state.planets.len()
    }

    /// Primary homeworld of one player.
    #[must_use]
    pub fn homeworld(&self, player: PlayerId) -> Option<&Planet> {
        self.homeworlds
            .get(&player)
            .and_then(|planet| self.state.planets.get(planet))
    }

    /// Planets owned by one player, in stable identifier order.
    pub fn owned_planets(&self, player: PlayerId) -> impl Iterator<Item = &Planet> {
        self.state
            .planets
            .values()
            .filter(move |planet| planet.owner == Some(player))
    }
}

/// A reason universe generation could not produce a state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniverseGenerationError {
    /// The settings fail their recovered ranges.
    InvalidSettings(Vec<UniverseSettingsError>),
    /// The human custom race fails field or point validation.
    InvalidRace(Vec<RaceValidationError>),
    /// There are fewer planets than player slots.
    NotEnoughPlanets {
        /// Generated planet count.
        planets: usize,
        /// Requested player count.
        players: usize,
    },
    /// No valid homeworld could be assigned to one player.
    HomeworldPlacementFailed {
        /// Player slot index, zero-based.
        player_index: usize,
    },
    /// A generator invariant failed after the world was built.
    InternalState(String),
}

impl std::fmt::Display for UniverseGenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSettings(errors) => {
                formatter.write_str("universe settings are invalid")?;
                for error in errors {
                    write!(formatter, ": {error}")?;
                }
                Ok(())
            }
            Self::InvalidRace(errors) => {
                formatter.write_str("custom race is invalid")?;
                for error in errors {
                    write!(formatter, ": {error}")?;
                }
                Ok(())
            }
            Self::NotEnoughPlanets { planets, players } => write!(
                formatter,
                "generated {planets} planets for {players} player slots"
            ),
            Self::HomeworldPlacementFailed { player_index } => {
                write!(
                    formatter,
                    "could not place homeworld for player slot {player_index}"
                )
            }
            Self::InternalState(message) => {
                write!(formatter, "universe generator invariant failed: {message}")
            }
        }
    }
}

impl std::error::Error for UniverseGenerationError {}

/// Generates a complete starting state.
///
/// # Errors
///
/// Returns typed settings, race or placement errors. No partial state is
/// returned on failure.
pub fn generate_universe(
    settings: &UniverseSettings,
    human_race: &RaceSpec,
) -> Result<GeneratedUniverse, UniverseGenerationError> {
    generate_universe_with_progress(settings, human_race, |_| {})
}

/// Generates a complete starting state and reports progress checkpoints.
///
/// The callback receives one checkpoint per [`GENERATION_STEPS`] stage. Core
/// generation does not know about worker threads or request identifiers; the
/// desktop worker maps these checkpoints to its request messages.
///
/// # Errors
///
/// Returns the same errors as [`generate_universe`].
pub fn generate_universe_with_progress(
    settings: &UniverseSettings,
    human_race: &RaceSpec,
    mut progress: impl FnMut(UniverseProgress),
) -> Result<GeneratedUniverse, UniverseGenerationError> {
    let mut step = 0_u32;
    let mut checkpoint = |stage, step: &mut u32| {
        *step += 1;
        progress(UniverseProgress {
            stage,
            completed_steps: *step,
            total_steps: GENERATION_STEPS,
        });
    };

    settings
        .validate()
        .map_err(UniverseGenerationError::InvalidSettings)?;
    let evaluation = validate_race(human_race, &ReferenceHabitability)
        .map_err(UniverseGenerationError::InvalidRace)?;
    checkpoint(UniverseGenerationStage::Validating, &mut step);

    let mut rng = GenerationRng::new(settings.seed);
    let mut seeds = generate_planet_seeds(settings, &mut rng)?;
    checkpoint(UniverseGenerationStage::Positions, &mut step);

    fill_planet_details(settings, &mut seeds, &mut rng);
    checkpoint(UniverseGenerationStage::Environments, &mut step);

    let player_count = usize::from(settings.player_count());
    let home_indices = place_homeworlds(
        &mut rng,
        &seeds,
        player_count,
        settings.size.span_ly(),
        settings.player_positions.stored_value(),
    )?;
    checkpoint(UniverseGenerationStage::Homeworlds, &mut step);

    let mut state = GameState::new(2400, settings.seed);
    let human = allocate_player(
        &mut state,
        human_race.name.clone(),
        PlayerRole::Human,
        human_race.clone(),
        starting_technology(human_race),
    )?;
    let mut opponents = Vec::with_capacity(player_count.saturating_sub(1));
    for index in 0..player_count.saturating_sub(1) {
        let race = turindrones_race(settings.difficulty);
        let name = if player_count == 2 {
            "Turindrones".to_owned()
        } else {
            format!("Turindrones {}", index + 1)
        };
        let technology = starting_technology(&race);
        let player = allocate_player(&mut state, name, PlayerRole::Computer, race, technology)?;
        opponents.push(player);
    }

    let mut planets = Vec::with_capacity(seeds.len());
    for (index, seed) in seeds.iter().enumerate() {
        let id = state
            .allocate_planet_id()
            .map_err(|error| UniverseGenerationError::InternalState(error.to_string()))?;
        planets.push(Planet {
            id,
            name: format!("Star {:04}", index + 1),
            position: seed.position,
            owner: None,
            visibility: EntityVisibility::Public,
            environment: seed.environment,
            mineral_concentration: seed.mineral_concentration,
            surface_minerals: [0; 3],
            population: 0,
            factories: 0,
            mines: 0,
            defenses: 0,
            homeworld: false,
            starbase: false,
        });
    }

    let mut all_players = vec![(human, human_race.clone(), evaluation.displayed)];
    for opponent in &opponents {
        let race = state
            .players
            .get(opponent)
            .and_then(|player| player.race.clone())
            .unwrap_or_else(|| turindrones_race(settings.difficulty));
        let points = evaluate_race(&race, &ReferenceHabitability).displayed;
        all_players.push((*opponent, race, points));
    }

    let template_surface = seeds.first().map_or([0; 3], |seed| seed.surface_minerals);
    let template_concentration = seeds.first().map_or([30; 3], |seed| {
        seed.mineral_concentration.map(|value| value.max(30))
    });
    let mut homeworlds = BTreeMap::new();
    let mut used = BTreeSet::new();
    for (slot, (player, race, points)) in all_players.iter().enumerate() {
        let index = home_indices[slot];
        used.insert(index);
        let planet = planets
            .get_mut(index)
            .ok_or(UniverseGenerationError::HomeworldPlacementFailed { player_index: slot })?;
        initialize_homeworld(
            settings,
            &mut rng,
            planet,
            player,
            race,
            *points,
            template_surface,
            template_concentration,
            (slot > 0).then_some(settings.difficulty),
        );
        homeworlds.insert(*player, planet.id);
    }

    for (slot, (player, race, _)) in all_players.iter().enumerate() {
        if matches!(
            race.primary,
            PrimaryTrait::InterstellarTraveler | PrimaryTrait::PacketPhysics
        ) && settings.size != crate::universe::settings::GalaxySize::Tiny
        {
            give_second_start_world(
                settings,
                &mut rng,
                &mut planets,
                &used,
                home_indices[slot],
                *player,
                race,
            );
        }
    }
    checkpoint(UniverseGenerationStage::Players, &mut step);

    for planet in planets {
        state.planets.insert(planet.id, planet);
    }
    state
        .validate_integrity()
        .map_err(|error| UniverseGenerationError::InternalState(error.to_string()))?;

    checkpoint(UniverseGenerationStage::Complete, &mut step);

    Ok(GeneratedUniverse {
        settings: settings.clone(),
        state,
        human,
        opponents,
        homeworlds,
    })
}

fn allocate_player(
    state: &mut GameState,
    name: String,
    role: PlayerRole,
    race: RaceSpec,
    starting_technology: [u8; 6],
) -> Result<PlayerId, UniverseGenerationError> {
    let id = state
        .allocate_player_id()
        .map_err(|error| UniverseGenerationError::InternalState(error.to_string()))?;
    state.players.insert(
        id,
        Player {
            id,
            name,
            role,
            ready: false,
            race: Some(race),
            starting_technology,
        },
    );
    Ok(id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlanetSeed {
    position: GalacticPosition,
    environment: PlanetEnvironment,
    mineral_concentration: [u8; 3],
    surface_minerals: [u32; 3],
}

fn generate_planet_seeds(
    settings: &UniverseSettings,
    rng: &mut GenerationRng,
) -> Result<Vec<PlanetSeed>, UniverseGenerationError> {
    let span = settings.size.span_ly();
    let target = usize::from(settings.planet_count());
    let candidate_count =
        usize::from((settings.planet_count() + settings.planet_count() / 7).min(999));
    if candidate_count == 0 {
        return Err(UniverseGenerationError::NotEnoughPlanets {
            planets: 0,
            players: usize::from(settings.player_count()),
        });
    }

    let mut candidates = Vec::with_capacity(candidate_count);
    for _ in 0..candidate_count {
        let x = 1_010 + rng.random(span + 1 - 20);
        let y = 1_010 + rng.random(span + 1 - 20);
        candidates.push(Point { x, y });
    }
    candidates.sort_by_key(|point| point.x);

    let mut alive = vec![true; candidates.len()];
    let mut killed = 0_usize;
    for index in 0..candidates.len() {
        if !alive[index] {
            continue;
        }
        let point = candidates[index];
        for other in (index + 1)..candidates.len() {
            if candidates[other].x > point.x + 12 {
                break;
            }
            if !alive[other] || (candidates[other].y - point.y).abs() > 12 {
                continue;
            }
            let dx = candidates[other].x - point.x;
            let dy = candidates[other].y - point.y;
            if dx * dx + dy * dy <= 12 * 12 {
                alive[other] = false;
                killed += 1;
            }
        }
    }

    let target_kills = candidates.len().saturating_sub(target);
    while killed < target_kills {
        let index = rng.random(i32::try_from(candidates.len()).unwrap_or(i32::MAX)) as usize;
        if !alive[index] {
            continue;
        }
        alive[index] = false;
        killed += 1;
    }

    let mut points: Vec<Point> = candidates
        .into_iter()
        .zip(alive)
        .filter_map(|(point, keep)| keep.then_some(point))
        .collect();
    if points.len() < usize::from(settings.player_count()) {
        return Err(UniverseGenerationError::NotEnoughPlanets {
            planets: points.len(),
            players: usize::from(settings.player_count()),
        });
    }

    if settings.clumping {
        let count = i32::try_from(points.len()).unwrap_or(i32::MAX);
        for _ in 0..count {
            let origin = rng.random(count) as usize;
            let mut best = 0_usize;
            let mut best_distance = i64::MAX;
            let point = points[origin];
            for (index, candidate) in points.iter().enumerate() {
                if index == origin {
                    continue;
                }
                let dx = i64::from(point.x - candidate.x);
                let dy = i64::from(point.y - candidate.y);
                let distance = dx * dx + dy * dy;
                if distance < best_distance {
                    best_distance = distance;
                    best = index;
                }
            }
            if best_distance > 12 * 12 {
                let nearest = points[best];
                let (old_x, old_y) = (points[origin].x, points[origin].y);
                let (new_x, new_y) = if best_distance > 40 * 40 {
                    ((old_x + 2 * nearest.x) / 3, (old_y + 2 * nearest.y) / 3)
                } else if best_distance > 25 * 25 {
                    ((old_x + nearest.x) / 2, (old_y + nearest.y) / 2)
                } else if best_distance > 18 * 18 {
                    ((2 * old_x + nearest.x) / 3, (2 * old_y + nearest.y) / 3)
                } else {
                    ((4 * old_x + nearest.x) / 5, (4 * old_y + nearest.y) / 5)
                };
                points[origin].x = new_x;
                points[origin].y = new_y;
            }
        }
        points.sort_by_key(|point| point.x);
    }

    Ok(points
        .into_iter()
        .map(|point| PlanetSeed {
            position: GalacticPosition::new(point.x, point.y),
            environment: PlanetEnvironment::new(1, 1, 1),
            mineral_concentration: [0; 3],
            surface_minerals: [0; 3],
        })
        .collect())
}

fn fill_planet_details(
    settings: &UniverseSettings,
    seeds: &mut [PlanetSeed],
    rng: &mut GenerationRng,
) {
    for seed in seeds.iter_mut() {
        let gravity = (1 + rng.random(90) + rng.random(10)) as u8;
        let temperature = (1 + rng.random(90) + rng.random(10)) as u8;
        let radiation = (1 + rng.random(99)) as u8;
        seed.environment = PlanetEnvironment::new(gravity, temperature, radiation);

        for concentration in &mut seed.mineral_concentration {
            if settings.options.maximum_minerals {
                *concentration = 100;
            } else {
                let mut value = (rng.random(45) + rng.random(45) + 31) as u8;
                if radiation >= 90 {
                    value = value.saturating_add((rng.random(99 - i32::from(value)) / 2) as u8);
                }
                if settings.options.accelerated_bbs && value < 40 {
                    value = value.saturating_add(5);
                }
                *concentration = value;
            }
        }

        let mut adjustment = if settings.options.maximum_minerals {
            100
        } else {
            rng.random(27)
        };
        if adjustment < 18 {
            if adjustment >= 9 {
                let value = (rng.random(30) + 1) as u8;
                let axis = rng.random(3) as usize;
                seed.mineral_concentration[axis] = value;
            } else {
                adjustment += 1;
                while adjustment < 16 {
                    let value = (rng.random(30) + 1) as u8;
                    let axis = rng.random(3) as usize;
                    seed.mineral_concentration[axis] = value;
                    adjustment <<= 1;
                }
            }
        }
    }

    if let Some(first) = seeds.first_mut() {
        for (axis, concentration) in first.mineral_concentration.iter().enumerate() {
            let mut value = rng.random(i32::from(*concentration) * 10) + 10;
            if value < 200 {
                value += 155 + rng.random(150);
            }
            if settings.options.accelerated_bbs {
                value += value / 4;
            }
            first.surface_minerals[axis] = value.max(0) as u32;
        }
    }
}

fn place_homeworlds(
    rng: &mut GenerationRng,
    seeds: &[PlanetSeed],
    player_count: usize,
    span: i32,
    start_distance: u8,
) -> Result<Vec<usize>, UniverseGenerationError> {
    if seeds.len() < player_count {
        return Err(UniverseGenerationError::NotEnoughPlanets {
            planets: seeds.len(),
            players: player_count,
        });
    }

    let count = i32::try_from(seeds.len()).unwrap_or(i32::MAX);
    let minimum = span * 6;
    let mut ideal = span * span / i32::try_from(player_count).unwrap_or(i32::MAX) - minimum;
    ideal = if ideal < 0 { 0 } else { ideal * 9 / 10 };
    ideal = ideal * i32::from(start_distance) / 3 + minimum;
    let mut minimum_distance = ideal * 9 / 10;
    let mut maximum_distance = ideal * 7 / 6;

    let center_min = span / 4 + 1_000;
    let center_max = span * 3 / 4 + 1_000;
    let mut best = 0_usize;
    let mut best_distance = i32::MAX;
    let mut center_choice = None;
    for _ in 0..50 {
        let index = rng.random(count) as usize;
        let point = seeds[index].position;
        let dx = if point.x < center_min {
            center_min - point.x
        } else if point.x > center_max {
            point.x - center_max
        } else {
            0
        };
        let dy = if point.y < center_min {
            center_min - point.y
        } else if point.y > center_max {
            point.y - center_max
        } else {
            0
        };
        if dx == 0 && dy == 0 {
            center_choice = Some(index);
            break;
        }
        let distance = dx * dx + dy * dy;
        if distance < best_distance {
            best_distance = distance;
            best = index;
        }
    }

    let mut chosen = vec![center_choice.unwrap_or(best)];
    let mut used = vec![false; seeds.len()];
    if let Some(slot) = used.get_mut(chosen[0]) {
        *slot = true;
    }

    let (distance_min, distance_max) = if player_count > 4 {
        (span / 20 + 1_000, span * 19 / 20 + 1_000)
    } else if player_count > 2 {
        (span / 10 + 1_000, span * 9 / 10 + 1_000)
    } else {
        (span * 3 / 20 + 1_000, span * 17 / 20 + 1_000)
    };

    for slot in 1..player_count {
        let mut placed = None;
        let mut last = 0_usize;
        for _ in 0..50 {
            let index = rng.random(count) as usize;
            last = index;
            if used[index] {
                continue;
            }
            let point = seeds[index].position;
            if !in_bounds(point, distance_min, distance_max) {
                continue;
            }
            if acceptable_distance(index, &chosen, seeds, minimum_distance, maximum_distance) {
                placed = Some(index);
                break;
            }
        }

        if placed.is_none() {
            let mut attempts = 0;
            loop {
                let mut found = None;
                for step in 1..=seeds.len() {
                    let index = (last + step) % seeds.len();
                    if used[index] {
                        continue;
                    }
                    let point = seeds[index].position;
                    if in_bounds(point, distance_min, distance_max)
                        && acceptable_distance(
                            index,
                            &chosen,
                            seeds,
                            minimum_distance,
                            maximum_distance,
                        )
                    {
                        found = Some(index);
                        break;
                    }
                }
                if let Some(index) = found {
                    placed = Some(index);
                    break;
                }
                let delta = (ideal / 35).max(1);
                minimum_distance = (minimum_distance - delta).max(0);
                maximum_distance += delta;
                attempts += 1;
                if attempts > 64 {
                    break;
                }
            }
        }

        let index = placed
            .or_else(|| {
                (0..seeds.len()).find(|index| {
                    !used[*index] && in_bounds(seeds[*index].position, distance_min, distance_max)
                })
            })
            .or_else(|| (0..seeds.len()).find(|index| !used[*index]))
            .ok_or(UniverseGenerationError::HomeworldPlacementFailed { player_index: slot })?;

        used[index] = true;
        chosen.push(index);
    }

    Ok(chosen)
}

fn acceptable_distance(
    candidate: usize,
    chosen: &[usize],
    seeds: &[PlanetSeed],
    minimum: i32,
    maximum: i32,
) -> bool {
    let point = seeds[candidate].position;
    let mut within_maximum = false;
    for previous in chosen {
        let other = seeds[*previous].position;
        let dx = point.x - other.x;
        let dy = point.y - other.y;
        let distance = dx * dx + dy * dy;
        if distance < 1 || distance < minimum {
            return false;
        }
        if distance <= maximum {
            within_maximum = true;
        }
    }
    within_maximum
}

fn in_bounds(point: GalacticPosition, minimum: i32, maximum: i32) -> bool {
    minimum <= point.x && point.x <= maximum && minimum <= point.y && point.y <= maximum
}

#[allow(clippy::too_many_arguments)]
fn initialize_homeworld(
    settings: &UniverseSettings,
    rng: &mut GenerationRng,
    planet: &mut Planet,
    player: &PlayerId,
    race: &RaceSpec,
    advantage_points: i32,
    template_surface: [u32; 3],
    template_concentration: [u8; 3],
    ai_difficulty: Option<Difficulty>,
) {
    planet.owner = Some(*player);
    planet.visibility = EntityVisibility::OwnerOnly;
    planet.homeworld = true;
    planet.starbase = true;
    planet.environment = home_environment(rng, race);
    planet.mineral_concentration = template_concentration;
    planet.surface_minerals = template_surface;

    let mut population = if race.has_lesser(LesserTrait::LowStartingPopulation) {
        17_500
    } else {
        25_000
    };
    let expert_ai = ai_difficulty == Some(Difficulty::Expert);
    let mut points = advantage_points.clamp(0, 50);
    if expert_ai {
        points = 50;
        population += population / 10;
    }
    if settings.options.accelerated_bbs {
        let growth = if race.primary == PrimaryTrait::HyperExpansion {
            i32::from(race.growth_rate) * 2
        } else {
            i32::from(race.growth_rate)
        };
        population = population * (growth * 2 + 10) / 10;
    }
    planet.population = population.max(0) as u32;

    let mut factories = 10_u32;
    let mut mines = 10_u32;
    let mut defenses = 10_u32;
    // `InitNewGamePlr()` sends Tough and Expert computer players to the
    // concentration branch regardless of their stored leftover choice.
    let allocation = if matches!(ai_difficulty, Some(Difficulty::Tough | Difficulty::Expert)) {
        crate::race::LeftoverAllocation::MineralConcentration
    } else {
        race.leftover
    };
    match allocation {
        crate::race::LeftoverAllocation::SurfaceMinerals => {
            let total = points * 10;
            let share = total / 4;
            let remainder = u32::try_from(total % 4).unwrap_or(0);
            let low = lowest_mineral(planet.surface_minerals);
            planet.surface_minerals[low] += u32::try_from(share).unwrap_or(0) + remainder;
            for value in &mut planet.surface_minerals {
                *value += u32::try_from(share).unwrap_or(0);
            }
        }
        crate::race::LeftoverAllocation::MineralConcentration => {
            let mut amount = if points > 0 && points < 3 {
                1
            } else {
                points / 2
            };
            let low = lowest_concentration(planet.mineral_concentration);
            planet.mineral_concentration[low] =
                planet.mineral_concentration[low].saturating_add(amount as u8);
            amount = (amount + 1) / 2;
            for value in &mut planet.mineral_concentration {
                *value = value.saturating_add(amount as u8);
            }
        }
        crate::race::LeftoverAllocation::Mines => {
            mines += u32::try_from(points / 2).unwrap_or(0);
        }
        crate::race::LeftoverAllocation::Factories => {
            factories += u32::try_from(points / 5).unwrap_or(0);
        }
        crate::race::LeftoverAllocation::Defenses => {
            defenses += u32::try_from((points + 5) / 10).unwrap_or(0);
        }
    }

    if race.primary == PrimaryTrait::AlternateReality {
        factories = 0;
        mines = 0;
        defenses = 0;
    }
    planet.factories = factories;
    planet.mines = mines;
    planet.defenses = defenses;
}

fn home_environment(rng: &mut GenerationRng, race: &RaceSpec) -> PlanetEnvironment {
    let mut values = [0_u8; 3];
    for (axis, value) in values.iter_mut().enumerate() {
        let band = race.habitability[axis];
        *value = if band.immune {
            (rng.random(99) + 1) as u8
        } else {
            u8::try_from(band.ideal().max(0)).unwrap_or(0)
        };
    }
    PlanetEnvironment::new(values[0], values[1], values[2])
}

fn lowest_mineral(values: [u32; 3]) -> usize {
    let mut lowest = 0;
    for index in 1..3 {
        if values[index] < values[lowest] {
            lowest = index;
        }
    }
    lowest
}

fn lowest_concentration(values: [u8; 3]) -> usize {
    let mut lowest = 0;
    for index in 1..3 {
        if values[index] < values[lowest] {
            lowest = index;
        }
    }
    lowest
}

#[allow(clippy::too_many_arguments)]
fn give_second_start_world(
    settings: &UniverseSettings,
    rng: &mut GenerationRng,
    planets: &mut [Planet],
    used: &BTreeSet<usize>,
    home_index: usize,
    player: PlayerId,
    race: &RaceSpec,
) {
    let span = settings.size.span_ly();
    let home = planets[home_index].position;
    let minimum = (span * 15 / 100) * (span * 15 / 100);
    let maximum = (span * 23 / 100) * (span * 23 / 100);
    let mut picked = None;
    let mut closest = None;
    let mut closest_distance = i32::MAX;
    let mut fit = 0_i32;

    for (index, planet) in planets.iter().enumerate() {
        if used.contains(&index) || planet.owner.is_some() {
            continue;
        }
        let dx = planet.position.x - home.x;
        let dy = planet.position.y - home.y;
        let distance = dx * dx + dy * dy;
        if (minimum..=maximum).contains(&distance) {
            fit += 1;
            if rng.random(fit) == 0 {
                picked = Some(index);
            }
        } else if picked.is_none() && distance < closest_distance {
            closest_distance = distance;
            closest = Some(index);
        }
    }

    let Some(index) = picked.or(closest) else {
        return;
    };
    let home_environment = planets[home_index].environment;
    let home_population = planets[home_index].population;
    let Some(planet) = planets.get_mut(index) else {
        return;
    };

    let mut attempts = 0;
    while reference_desirability(planet.environment.signed_values(), race) < 10 {
        if attempts >= 100 {
            break;
        }
        attempts += 1;
        planet.environment = PlanetEnvironment::new(
            (rng.random(97) + 2) as u8,
            (rng.random(97) + 2) as u8,
            (rng.random(97) + 2) as u8,
        );
    }
    if attempts >= 100 {
        planet.environment = home_environment;
    }

    planet.population = home_population.saturating_mul(2) / 5;
    planet.owner = Some(player);
    planet.visibility = EntityVisibility::OwnerOnly;
    planet.starbase = true;
    planet.surface_minerals = [
        (rng.random(200) + 100) as u32,
        (rng.random(200) + 100) as u32,
        (rng.random(200) + 100) as u32,
    ];
    planets[home_index].population = home_population.saturating_mul(4) / 5;
}

/// Recovers the starting technology from `InitNewGamePlr()`.
///
/// Primary-trait entries, Tech 3, Cheap Engines and Improved Fuel Efficiency
/// are applied in the same order as the decompiled source.
#[must_use]
pub fn starting_technology(race: &RaceSpec) -> [u8; 6] {
    let mut technology = [0_u8; 6];
    match race.primary {
        PrimaryTrait::SuperStealth => technology[4] = 5,
        PrimaryTrait::WarMonger => {
            technology[1] = 6;
            technology[2] = 1;
            technology[0] = 1;
        }
        PrimaryTrait::ClaimAdjuster => {
            technology[5] = 6;
            technology[3] = 2;
            technology[0] = 1;
            technology[1] = 1;
            technology[2] = 1;
        }
        PrimaryTrait::SpaceDemolition => {
            technology[2] = 2;
            technology[5] = 2;
        }
        PrimaryTrait::PacketPhysics => technology[0] = 4,
        PrimaryTrait::InterstellarTraveler => {
            technology[2] = 5;
            technology[3] = 5;
        }
        PrimaryTrait::AlternateReality => technology[0] = 1,
        PrimaryTrait::JackOfAllTrades => technology = [3; 6],
        PrimaryTrait::HyperExpansion | PrimaryTrait::InnerStrength => {}
    }

    if race.research.tech3 {
        let minimum = if race.primary == PrimaryTrait::JackOfAllTrades {
            4
        } else {
            3
        };
        for (index, cost) in race.research.costs.iter().enumerate() {
            if technology[index] < minimum && *cost == ResearchCost::Extra {
                technology[index] = minimum;
            }
        }
    }
    if race.has_lesser(LesserTrait::CheapEngines) {
        technology[2] = technology[2].saturating_add(1);
    }
    if race.has_lesser(LesserTrait::ImprovedFuelEfficiency) {
        technology[2] = technology[2].saturating_add(1);
    }
    technology
}

struct GenerationRng {
    seed: u64,
    draw: u64,
}

impl GenerationRng {
    fn new(seed: u64) -> Self {
        Self {
            seed: mix64(seed ^ 0x5354_4152_5355_4e49),
            draw: 0,
        }
    }

    fn random(&mut self, count: i32) -> i32 {
        if count <= 0 {
            return 0;
        }
        let value = mix64(self.seed ^ self.draw.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        self.draw = self.draw.wrapping_add(1);
        i32::try_from(value % u64::try_from(count).unwrap_or(1)).unwrap_or(0)
    }
}

// These imports are part of the generated contract's documented bounds.
const _: () = {
    assert!(MAX_OPPONENTS == MAX_PLAYERS - 1);
    assert!(GAME_NAME_MAX_BYTES == 31);
};

#[cfg(test)]
mod tests {
    use super::{GenerationRng, starting_technology};
    use crate::race::{LesserTrait, PrimaryTrait, RaceSpec, ResearchCost};
    use crate::universe::settings::Difficulty;

    #[test]
    fn generation_rng_is_reproducible_and_range_bounded() {
        let mut first = GenerationRng::new(42);
        let mut second = GenerationRng::new(42);
        for _ in 0..100 {
            let value = first.random(7);
            assert_eq!(value, second.random(7));
            assert!((0..7).contains(&value));
        }
    }

    #[test]
    fn starting_technology_matches_the_recovered_primary_entries() {
        let mut race = RaceSpec::humanoids_anchor();
        race.primary = PrimaryTrait::WarMonger;
        assert_eq!(starting_technology(&race), [1, 6, 1, 0, 0, 0]);

        race.primary = PrimaryTrait::JackOfAllTrades;
        race.research.costs = [ResearchCost::Extra; 6];
        race.research.tech3 = true;
        assert_eq!(starting_technology(&race), [4, 4, 4, 4, 4, 4]);

        race.primary = PrimaryTrait::SuperStealth;
        race.research.costs = [ResearchCost::Standard; 6];
        race.research.tech3 = false;
        race.lesser.insert(LesserTrait::ImprovedFuelEfficiency);
        race.lesser.insert(LesserTrait::CheapEngines);
        assert_eq!(starting_technology(&race), [0, 0, 2, 0, 5, 0]);
        assert_eq!(Difficulty::Expert.stored_value(), 3);
    }
}
