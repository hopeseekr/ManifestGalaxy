//! Typed custom-race specification and editable draft.
//!
//! Field ranges come from `rgRaceStatMin`/`rgRaceStatMax`, the environment
//! fields in `stars-decompile/types.h`, and the recovered wizard inventory.
//! The environment ideal point is derived as the band midpoint, matching
//! `BoundsCheckPlayer()`; the wizard exposes the band and immunity, not a
//! separate ideal-point control.

use std::collections::BTreeSet;

use crate::race::traits::{LesserTrait, PrimaryTrait};
use crate::race::validate::RaceValidationError;

/// Lowest accepted environment value.
pub const ENVIRONMENT_MIN: u8 = 0;
/// Highest accepted environment value.
pub const ENVIRONMENT_MAX: u8 = 100;
/// Lowest accepted growth rate in percent per year.
pub const GROWTH_MIN: u8 = 1;
/// Highest accepted growth rate in percent per year.
pub const GROWTH_MAX: u8 = 20;
/// Maximum race or plural name length in bytes, from the 32-byte `PLAYER`
/// name fields including their terminator.
pub const RACE_NAME_MAX_BYTES: usize = 31;
/// Maximum emblem index, from the five-bit `iPlrBmp` field.
pub const EMBLEM_MAX: u8 = 31;

/// Inclusive bounds for a race efficiency attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttributeBounds {
    /// Minimum accepted value.
    pub min: u8,
    /// Maximum accepted value.
    pub max: u8,
}

impl AttributeBounds {
    /// Creates an inclusive range.
    #[must_use]
    pub const fn new(min: u8, max: u8) -> Self {
        Self { min, max }
    }

    /// Whether a value lies in the range.
    #[must_use]
    pub const fn contains(self, value: u8) -> bool {
        value >= self.min && value <= self.max
    }
}

/// Bounds for colonist resource generation (`rsResGen`).
pub const RESOURCE_GENERATION_BOUNDS: AttributeBounds = AttributeBounds::new(7, 25);
/// Bounds for factory production (`rsFactProd`).
pub const FACTORY_PRODUCTION_BOUNDS: AttributeBounds = AttributeBounds::new(5, 15);
/// Bounds for factory build cost (`rsFactBuild`).
pub const FACTORY_COST_BOUNDS: AttributeBounds = AttributeBounds::new(5, 25);
/// Bounds for factory operation (`rsFactOperate`).
pub const FACTORY_OPERATION_BOUNDS: AttributeBounds = AttributeBounds::new(5, 25);
/// Bounds for mine production (`rsMineProd`).
pub const MINE_PRODUCTION_BOUNDS: AttributeBounds = AttributeBounds::new(5, 25);
/// Bounds for mine build cost (`rsMineBuild`).
pub const MINE_COST_BOUNDS: AttributeBounds = AttributeBounds::new(2, 15);
/// Bounds for mine operation (`rsMineOperate`).
pub const MINE_OPERATION_BOUNDS: AttributeBounds = AttributeBounds::new(5, 25);

/// One environmental band.
///
/// `min` and `max` are retained even when the axis is immune so the editor can
/// restore them if immunity is cleared. The effective bounds and ideal point
/// are `-1` when immune.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HabitabilityRange {
    /// Inclusive lower bound.
    pub min: u8,
    /// Inclusive upper bound.
    pub max: u8,
    /// Whether the race is immune to this environment axis.
    pub immune: bool,
}

impl HabitabilityRange {
    /// Creates a non-immune band.
    #[must_use]
    pub const fn new(min: u8, max: u8) -> Self {
        Self {
            min,
            max,
            immune: false,
        }
    }

    /// Creates an immune axis. Stored bounds remain available to the editor.
    #[must_use]
    pub const fn immune_with_bounds(min: u8, max: u8) -> Self {
        Self {
            min,
            max,
            immune: true,
        }
    }

    /// Ideal point used by the point calculation: the band midpoint, or `-1`
    /// when immune.
    #[must_use]
    pub const fn ideal(self) -> i8 {
        if self.immune {
            -1
        } else {
            (self.min + (self.max - self.min) / 2) as i8
        }
    }

    /// Effective lower bound for the point calculation.
    #[must_use]
    pub const fn effective_min(self) -> i8 {
        if self.immune { -1 } else { self.min as i8 }
    }

    /// Effective upper bound for the point calculation.
    #[must_use]
    pub const fn effective_max(self) -> i8 {
        if self.immune { -1 } else { self.max as i8 }
    }
}

impl Default for HabitabilityRange {
    fn default() -> Self {
        Self::new(15, 85)
    }
}

/// The seven economic efficiency attributes recovered from `RaceStat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Efficiency {
    /// Colonists per resource point.
    pub resource_generation: u8,
    /// Factories produced per 10,000 colonists.
    pub factory_production: u8,
    /// Factory build cost.
    pub factory_cost: u8,
    /// Factories operated per 10,000 colonists.
    pub factory_operation: u8,
    /// Mines produced per 10,000 colonists.
    pub mine_production: u8,
    /// Mine build cost.
    pub mine_cost: u8,
    /// Mines operated per 10,000 colonists.
    pub mine_operation: u8,
}

impl Efficiency {
    /// Neutral builder values, using the recovered Humanoids-shaped defaults.
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            resource_generation: 10,
            factory_production: 10,
            factory_cost: 10,
            factory_operation: 10,
            mine_production: 10,
            mine_cost: 5,
            mine_operation: 10,
        }
    }
}

impl Default for Efficiency {
    fn default() -> Self {
        Self::neutral()
    }
}

/// One research-cost choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResearchCost {
    /// Costs 50% less.
    Cheaper,
    /// Standard cost.
    Standard,
    /// Costs 75% extra.
    Extra,
}

impl ResearchCost {
    /// The stored `rsTechBonus` value.
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::Cheaper => 0,
            Self::Standard => 1,
            Self::Extra => 2,
        }
    }

    /// Converts a persisted value back to a choice.
    #[must_use]
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Cheaper),
            1 => Some(Self::Standard),
            2 => Some(Self::Extra),
            _ => None,
        }
    }

    /// Builder label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cheaper => "Costs 50% less",
            Self::Standard => "Standard",
            Self::Extra => "Costs 75% extra",
        }
    }
}

/// Names of the six technology fields in wire order.
pub const RESEARCH_FIELD_NAMES: [&str; 6] = [
    "Energy",
    "Weapons",
    "Propulsion",
    "Construction",
    "Electronics",
    "Biotechnology",
];

/// Step 6 research settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResearchSettings {
    /// One choice per field, in [`RESEARCH_FIELD_NAMES`] order.
    pub costs: [ResearchCost; 6],
    /// Whether all fields start at technology 3 (the recovered Tech 3 flag).
    pub tech3: bool,
}

impl ResearchSettings {
    /// Standard costs and no starting-technology bonus.
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            costs: [ResearchCost::Standard; 6],
            tech3: false,
        }
    }
}

impl Default for ResearchSettings {
    fn default() -> Self {
        Self::standard()
    }
}

/// Leftover advantage-point target from Step 1.
///
/// The exact per-choice rounding belongs to PR6; PR5 only records the
/// selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeftoverAllocation {
    /// Surface minerals.
    SurfaceMinerals,
    /// Mines.
    Mines,
    /// Factories.
    Factories,
    /// Defenses.
    Defenses,
    /// Mineral concentration.
    MineralConcentration,
}

impl LeftoverAllocation {
    /// All five recovered choices.
    pub const ALL: [Self; 5] = [
        Self::SurfaceMinerals,
        Self::Mines,
        Self::Factories,
        Self::Defenses,
        Self::MineralConcentration,
    ];

    /// Builder label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SurfaceMinerals => "Surface minerals",
            Self::Mines => "Mines",
            Self::Factories => "Factories",
            Self::Defenses => "Defenses",
            Self::MineralConcentration => "Mineral concentration",
        }
    }

    /// Stored choice index (`rsUseLeftover`).
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::SurfaceMinerals => 0,
            Self::Mines => 1,
            Self::Factories => 2,
            Self::Defenses => 3,
            Self::MineralConcentration => 4,
        }
    }

    /// Converts a persisted index back to a choice.
    #[must_use]
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::SurfaceMinerals),
            1 => Some(Self::Mines),
            2 => Some(Self::Factories),
            3 => Some(Self::Defenses),
            4 => Some(Self::MineralConcentration),
            _ => None,
        }
    }
}

/// A validated or hand-constructed custom race.
///
/// The type stores exactly one [`PrimaryTrait`]; there is no representation of
/// "no primary trait". An editable draft uses [`RaceDraft`] instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceSpec {
    /// Race name.
    pub name: String,
    /// Plural race name.
    pub plural_name: String,
    /// Emblem index in `0..=31`.
    pub emblem: u8,
    /// Local race-file password from Step 1.
    ///
    /// This is an offline, single-player race-file field. It is not a security
    /// boundary and must not be logged; PR16 owns persistence.
    pub password: String,
    /// The single primary racial trait.
    pub primary: PrimaryTrait,
    /// Selected lesser traits.
    pub lesser: BTreeSet<LesserTrait>,
    /// Ideal growth rate in percent per year.
    pub growth_rate: u8,
    /// Gravity, temperature and radiation bands.
    pub habitability: [HabitabilityRange; 3],
    /// Economic efficiency attributes.
    pub efficiency: Efficiency,
    /// Whether factories cost one less (the recovered CheapFact option flag).
    pub cheap_factories: bool,
    /// Step 6 research settings.
    pub research: ResearchSettings,
    /// Leftover-point allocation choice.
    pub leftover: LeftoverAllocation,
}

impl RaceSpec {
    /// The decompilation's `vrgplrDef[0]` record used only as the documented
    /// advantage-point anchor. It is not a selectable preset race and is not
    /// offered by the builder.
    #[must_use]
    pub fn humanoids_anchor() -> Self {
        Self {
            name: "Humanoids".to_owned(),
            plural_name: "Humanoids".to_owned(),
            emblem: 1,
            password: String::new(),
            primary: PrimaryTrait::JackOfAllTrades,
            lesser: BTreeSet::new(),
            growth_rate: 15,
            habitability: [HabitabilityRange::new(15, 85); 3],
            efficiency: Efficiency::neutral(),
            cheap_factories: false,
            research: ResearchSettings::standard(),
            leftover: LeftoverAllocation::SurfaceMinerals,
        }
    }

    /// Ideal point for one axis.
    #[must_use]
    pub const fn ideal(&self, axis: usize) -> i8 {
        self.habitability[axis].ideal()
    }

    /// Whether a lesser trait is selected.
    #[must_use]
    pub fn has_lesser(&self, trait_value: LesserTrait) -> bool {
        self.lesser.contains(&trait_value)
    }
}

/// An editable, possibly incomplete race draft.
///
/// A draft may have no primary trait and may hold values outside the recovered
/// bounds so the editor can report field-specific errors. Only a draft that
/// converts to [`RaceSpec`] and passes `validate_race` may create a game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceDraft {
    /// Race name.
    pub name: String,
    /// Plural race name.
    pub plural_name: String,
    /// Emblem index.
    pub emblem: u8,
    /// Local race-file password from Step 1.
    pub password: String,
    /// Selected primary trait, if any.
    pub primary: Option<PrimaryTrait>,
    /// Selected lesser traits.
    pub lesser: BTreeSet<LesserTrait>,
    /// Ideal growth rate.
    pub growth_rate: u8,
    /// Environmental bands.
    pub habitability: [HabitabilityRange; 3],
    /// Economic efficiency attributes.
    pub efficiency: Efficiency,
    /// Whether factories cost one less.
    pub cheap_factories: bool,
    /// Research settings.
    pub research: ResearchSettings,
    /// Leftover allocation.
    pub leftover: LeftoverAllocation,
}

impl RaceDraft {
    /// A neutral draft with no primary trait selected.
    #[must_use]
    pub fn neutral() -> Self {
        Self {
            name: "New Race".to_owned(),
            plural_name: "New Races".to_owned(),
            emblem: 0,
            password: String::new(),
            primary: None,
            lesser: BTreeSet::new(),
            growth_rate: 15,
            habitability: [HabitabilityRange::default(); 3],
            efficiency: Efficiency::neutral(),
            cheap_factories: false,
            research: ResearchSettings::standard(),
            leftover: LeftoverAllocation::SurfaceMinerals,
        }
    }

    /// Creates a draft from a complete race.
    #[must_use]
    pub fn from_spec(spec: &RaceSpec) -> Self {
        Self {
            name: spec.name.clone(),
            plural_name: spec.plural_name.clone(),
            emblem: spec.emblem,
            password: spec.password.clone(),
            primary: Some(spec.primary),
            lesser: spec.lesser.clone(),
            growth_rate: spec.growth_rate,
            habitability: spec.habitability,
            efficiency: spec.efficiency,
            cheap_factories: spec.cheap_factories,
            research: spec.research,
            leftover: spec.leftover,
        }
    }

    /// Converts the draft to a complete race.
    ///
    /// This checks only structural completeness (a primary trait is present).
    /// Range checks and the advantage-point gate belong to `validate_race`.
    ///
    /// # Errors
    ///
    /// Returns a single `MissingPrimaryTrait` error while no primary trait is
    /// selected.
    pub fn to_spec(&self) -> Result<RaceSpec, Vec<RaceValidationError>> {
        let Some(primary) = self.primary else {
            return Err(vec![RaceValidationError::MissingPrimaryTrait]);
        };
        Ok(RaceSpec {
            name: self.name.clone(),
            plural_name: self.plural_name.clone(),
            emblem: self.emblem,
            password: self.password.clone(),
            primary,
            lesser: self.lesser.clone(),
            growth_rate: self.growth_rate,
            habitability: self.habitability,
            efficiency: self.efficiency,
            cheap_factories: self.cheap_factories,
            research: self.research,
            leftover: self.leftover,
        })
    }
}

impl Default for RaceDraft {
    fn default() -> Self {
        Self::neutral()
    }
}

#[cfg(test)]
mod tests {
    use super::{HabitabilityRange, RaceDraft, ResearchCost};

    #[test]
    fn ideal_points_are_band_midpoints_and_immunity_is_negative_one() {
        assert_eq!(HabitabilityRange::new(15, 85).ideal(), 50);
        assert_eq!(HabitabilityRange::new(0, 101).ideal(), 50);
        let immune = HabitabilityRange::immune_with_bounds(15, 85);
        assert_eq!(immune.ideal(), -1);
        assert_eq!(immune.effective_min(), -1);
        assert_eq!(immune.effective_max(), -1);
    }

    #[test]
    fn draft_without_primary_is_structurally_incomplete() {
        let draft = RaceDraft::neutral();
        let errors = draft.to_spec().expect_err("no primary");
        assert_eq!(
            errors,
            vec![super::RaceValidationError::MissingPrimaryTrait]
        );
    }

    #[test]
    fn research_codes_round_trip() {
        for cost in [
            ResearchCost::Cheaper,
            ResearchCost::Standard,
            ResearchCost::Extra,
        ] {
            assert_eq!(ResearchCost::from_value(cost.value()), Some(cost));
        }
        assert_eq!(ResearchCost::from_value(3), None);
    }
}
