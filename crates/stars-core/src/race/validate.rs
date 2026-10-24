//! Field-level validation for a complete custom race.
//!
//! The exclusion matrix was not recovered (`GAP-EXCLUSIONS-001`), so this
//! module deliberately does not reject any priced trait combination. It enforces
//! the recovered field ranges, one primary trait, a non-negative displayed
//! advantage-point balance and structural completeness.

use std::fmt;

use crate::race::habitability::HabitabilityModel;
use crate::race::points::{RaceEvaluation, evaluate_race};
use crate::race::spec::{
    EMBLEM_MAX, ENVIRONMENT_MAX, FACTORY_COST_BOUNDS, FACTORY_OPERATION_BOUNDS,
    FACTORY_PRODUCTION_BOUNDS, GROWTH_MAX, GROWTH_MIN, MINE_COST_BOUNDS, MINE_OPERATION_BOUNDS,
    MINE_PRODUCTION_BOUNDS, RACE_NAME_MAX_BYTES, RESOURCE_GENERATION_BOUNDS, RaceSpec,
    ResearchCost,
};

/// A builder field that can carry a validation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaceField {
    /// Race name.
    Name,
    /// Plural race name.
    PluralName,
    /// Emblem index.
    Emblem,
    /// Primary trait selection.
    Primary,
    /// Growth rate.
    GrowthRate,
    /// Gravity band.
    Gravity,
    /// Temperature band.
    Temperature,
    /// Radiation band.
    Radiation,
    /// Colonists per resource.
    ResourceGeneration,
    /// Factory production.
    FactoryProduction,
    /// Factory cost.
    FactoryCost,
    /// Factory operation.
    FactoryOperation,
    /// Mine production.
    MineProduction,
    /// Mine cost.
    MineCost,
    /// Mine operation.
    MineOperation,
    /// One research field, by index.
    Research(usize),
    /// Leftover-point allocation.
    Leftover,
    /// Advantage-point balance.
    AdvantagePoints,
}

impl RaceField {
    /// Stable key suitable for a UI lookup.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::PluralName => "plural_name",
            Self::Emblem => "emblem",
            Self::Primary => "primary",
            Self::GrowthRate => "growth_rate",
            Self::Gravity => "gravity",
            Self::Temperature => "temperature",
            Self::Radiation => "radiation",
            Self::ResourceGeneration => "resource_generation",
            Self::FactoryProduction => "factory_production",
            Self::FactoryCost => "factory_cost",
            Self::FactoryOperation => "factory_operation",
            Self::MineProduction => "mine_production",
            Self::MineCost => "mine_cost",
            Self::MineOperation => "mine_operation",
            Self::Research(0) => "research_energy",
            Self::Research(1) => "research_weapons",
            Self::Research(2) => "research_propulsion",
            Self::Research(3) => "research_construction",
            Self::Research(4) => "research_electronics",
            Self::Research(5) => "research_biotechnology",
            Self::Research(_) => "research_unknown",
            Self::Leftover => "leftover",
            Self::AdvantagePoints => "advantage_points",
        }
    }
}

/// A structured race-validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RaceValidationError {
    /// No primary trait was selected.
    MissingPrimaryTrait,
    /// A required display name is empty.
    BlankName {
        /// Offending field.
        field: RaceField,
    },
    /// A name exceeds the recovered 32-byte field, including its terminator.
    NameTooLong {
        /// Offending field.
        field: RaceField,
        /// Measured byte length.
        bytes: usize,
    },
    /// The emblem index exceeds the five-bit stored field.
    EmblemOutOfRange {
        /// Supplied value.
        value: u8,
    },
    /// Growth is outside the documented 1%-20% range.
    GrowthOutOfRange {
        /// Supplied value.
        value: u8,
    },
    /// An environmental band is inverted.
    HabitabilityInverted {
        /// Axis index: 0 gravity, 1 temperature, 2 radiation.
        axis: usize,
        /// Lower bound.
        min: u8,
        /// Upper bound.
        max: u8,
    },
    /// An environmental bound exceeds 100.
    HabitabilityOutOfRange {
        /// Axis index.
        axis: usize,
        /// Offending bound.
        value: u8,
    },
    /// An efficiency attribute is outside its recovered range.
    AttributeOutOfRange {
        /// Offending field.
        field: RaceField,
        /// Supplied value.
        value: u8,
        /// Minimum accepted value.
        min: u8,
        /// Maximum accepted value.
        max: u8,
    },
    /// The displayed advantage-point balance is below zero.
    NegativeBalance {
        /// Displayed balance.
        displayed: i32,
    },
}

impl RaceValidationError {
    /// Field this error should be attached to.
    #[must_use]
    pub const fn field(&self) -> RaceField {
        match self {
            Self::MissingPrimaryTrait => RaceField::Primary,
            Self::BlankName { field } | Self::NameTooLong { field, .. } => *field,
            Self::EmblemOutOfRange { .. } => RaceField::Emblem,
            Self::GrowthOutOfRange { .. } => RaceField::GrowthRate,
            Self::HabitabilityInverted { axis, .. } | Self::HabitabilityOutOfRange { axis, .. } => {
                match axis {
                    0 => RaceField::Gravity,
                    1 => RaceField::Temperature,
                    _ => RaceField::Radiation,
                }
            }
            Self::AttributeOutOfRange { field, .. } => *field,
            Self::NegativeBalance { .. } => RaceField::AdvantagePoints,
        }
    }

    /// Stable key for UI lookup.
    #[must_use]
    pub const fn field_key(&self) -> &'static str {
        self.field().key()
    }
}

impl fmt::Display for RaceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPrimaryTrait => {
                formatter.write_str("select exactly one primary racial trait")
            }
            Self::BlankName { field } => write!(formatter, "{} must not be empty", field.key()),
            Self::NameTooLong { field, bytes } => write!(
                formatter,
                "{} is {bytes} bytes; the limit is {RACE_NAME_MAX_BYTES}",
                field.key()
            ),
            Self::EmblemOutOfRange { value } => {
                write!(formatter, "emblem {value} is outside 0..={EMBLEM_MAX}")
            }
            Self::GrowthOutOfRange { value } => write!(
                formatter,
                "growth rate {value} is outside {GROWTH_MIN}..={GROWTH_MAX}"
            ),
            Self::HabitabilityInverted { axis, min, max } => write!(
                formatter,
                "environment axis {axis} has minimum {min} above maximum {max}"
            ),
            Self::HabitabilityOutOfRange { axis, value } => write!(
                formatter,
                "environment axis {axis} value {value} exceeds {ENVIRONMENT_MAX}"
            ),
            Self::AttributeOutOfRange {
                field,
                value,
                min,
                max,
            } => write!(
                formatter,
                "{} value {value} is outside {min}..={max}",
                field.key()
            ),
            Self::NegativeBalance { displayed } => write!(
                formatter,
                "advantage-point balance {displayed} is negative; add disadvantages or reduce advantages"
            ),
        }
    }
}

impl std::error::Error for RaceValidationError {}

/// Checks structural and numeric fields without evaluating points.
#[must_use]
pub fn validate_race_fields(spec: &RaceSpec) -> Vec<RaceValidationError> {
    let mut errors = Vec::new();

    check_name(RaceField::Name, &spec.name, &mut errors);
    check_name(RaceField::PluralName, &spec.plural_name, &mut errors);

    if spec.emblem > EMBLEM_MAX {
        errors.push(RaceValidationError::EmblemOutOfRange { value: spec.emblem });
    }

    if !(GROWTH_MIN..=GROWTH_MAX).contains(&spec.growth_rate) {
        errors.push(RaceValidationError::GrowthOutOfRange {
            value: spec.growth_rate,
        });
    }

    for (axis, band) in spec.habitability.iter().enumerate() {
        if band.min > ENVIRONMENT_MAX || band.max > ENVIRONMENT_MAX {
            let value = if band.min > ENVIRONMENT_MAX {
                band.min
            } else {
                band.max
            };
            errors.push(RaceValidationError::HabitabilityOutOfRange { axis, value });
        }
        if band.min > band.max {
            errors.push(RaceValidationError::HabitabilityInverted {
                axis,
                min: band.min,
                max: band.max,
            });
        }
    }

    check_attribute(
        RaceField::ResourceGeneration,
        spec.efficiency.resource_generation,
        RESOURCE_GENERATION_BOUNDS.min,
        RESOURCE_GENERATION_BOUNDS.max,
        &mut errors,
    );
    check_attribute(
        RaceField::FactoryProduction,
        spec.efficiency.factory_production,
        FACTORY_PRODUCTION_BOUNDS.min,
        FACTORY_PRODUCTION_BOUNDS.max,
        &mut errors,
    );
    check_attribute(
        RaceField::FactoryCost,
        spec.efficiency.factory_cost,
        FACTORY_COST_BOUNDS.min,
        FACTORY_COST_BOUNDS.max,
        &mut errors,
    );
    check_attribute(
        RaceField::FactoryOperation,
        spec.efficiency.factory_operation,
        FACTORY_OPERATION_BOUNDS.min,
        FACTORY_OPERATION_BOUNDS.max,
        &mut errors,
    );
    check_attribute(
        RaceField::MineProduction,
        spec.efficiency.mine_production,
        MINE_PRODUCTION_BOUNDS.min,
        MINE_PRODUCTION_BOUNDS.max,
        &mut errors,
    );
    check_attribute(
        RaceField::MineCost,
        spec.efficiency.mine_cost,
        MINE_COST_BOUNDS.min,
        MINE_COST_BOUNDS.max,
        &mut errors,
    );
    check_attribute(
        RaceField::MineOperation,
        spec.efficiency.mine_operation,
        MINE_OPERATION_BOUNDS.min,
        MINE_OPERATION_BOUNDS.max,
        &mut errors,
    );

    // ResearchCost is a closed enum, so every stored choice is valid. The loop
    // keeps the field inventory explicit for UI lookup.
    for (index, _) in spec.research.costs.iter().enumerate() {
        debug_assert!(ResearchCost::from_value(spec.research.costs[index].value()).is_some());
    }

    errors
}

fn check_name(field: RaceField, value: &str, errors: &mut Vec<RaceValidationError>) {
    if value.trim().is_empty() {
        errors.push(RaceValidationError::BlankName { field });
    }
    if value.len() > RACE_NAME_MAX_BYTES {
        errors.push(RaceValidationError::NameTooLong {
            field,
            bytes: value.len(),
        });
    }
}

fn check_attribute(
    field: RaceField,
    value: u8,
    min: u8,
    max: u8,
    errors: &mut Vec<RaceValidationError>,
) {
    if value < min || value > max {
        errors.push(RaceValidationError::AttributeOutOfRange {
            field,
            value,
            min,
            max,
        });
    }
}

/// Validates a complete race and evaluates its advantage points.
///
/// The exclusion matrix is unknown, so any priced lesser-trait combination is
/// accepted; count and mixture penalties still apply. The returned evaluation
/// is the same one the builder displays.
///
/// # Errors
///
/// Returns every field error found. If the numeric fields are valid but the
/// displayed balance is negative, the result contains only
/// [`RaceValidationError::NegativeBalance`].
pub fn validate_race(
    spec: &RaceSpec,
    habitability: &dyn HabitabilityModel,
) -> Result<RaceEvaluation, Vec<RaceValidationError>> {
    let field_errors = validate_race_fields(spec);
    if !field_errors.is_empty() {
        return Err(field_errors);
    }

    let evaluation = evaluate_race(spec, habitability);
    if evaluation.displayed < 0 {
        return Err(vec![RaceValidationError::NegativeBalance {
            displayed: evaluation.displayed,
        }]);
    }
    Ok(evaluation)
}
