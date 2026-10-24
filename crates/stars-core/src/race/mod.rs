//! Custom-race representation, validation, point evaluation and trait coverage.
//!
//! PR5 owns the builder-facing rules. Runtime trait effects belong to PR6-PR13
//! and are recorded in [`traits::PRIMARY_EFFECT_COVERAGE`] and
//! [`traits::LESSER_EFFECT_COVERAGE`] so selecting a trait never hides a
//! permanently missing ability.
//!
//! The advantage-point formula is recovered from `stars-decompile/race.c` and
//! documented in `docs/rules/advantage-points.md`. It is not an observation from
//! `stars26i.exe`. Trait exclusions remain unknown, so no combination is
//! rejected on a guessed rule.

pub mod habitability;
pub mod points;
pub mod spec;
pub mod traits;
pub mod validate;

pub use habitability::{
    FixedHabitability, HabitabilityModel, ReferenceHabitability, innate_habitability_with,
    reference_desirability,
};
pub use points::{PointTerm, RaceEvaluation, evaluate_race, rule_id};
pub use spec::{
    AttributeBounds, Efficiency, HABITABILITY_MIN_SPAN, HabitabilityRange, LeftoverAllocation,
    RaceDraft, RaceSpec, ResearchCost, ResearchSettings,
};
pub use traits::{
    CostEvidence, LESSER_EFFECT_COVERAGE, LesserTrait, PRIMARY_EFFECT_COVERAGE, PrimaryTrait,
    TraitCost, TraitEffectCoverage,
};
pub use validate::{RaceField, RaceValidationError, validate_race, validate_race_fields};

use crate::model::{
    CanonicalEncode, CanonicalReader, StateDecodeError, StateDecodeErrorKind, encode_string,
};

impl CanonicalEncode for RaceSpec {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_string(out, &self.name);
        encode_string(out, &self.plural_name);
        out.push(self.emblem);
        encode_string(out, &self.password);
        out.push(self.primary.attribute_index());

        let mut lesser_bits = 0_u16;
        for trait_value in &self.lesser {
            lesser_bits |= 1_u16 << trait_value.bit_index();
        }
        out.extend_from_slice(&lesser_bits.to_le_bytes());

        out.push(self.growth_rate);
        for band in self.habitability {
            out.push(band.min);
            out.push(band.max);
            out.push(u8::from(band.immune));
        }

        out.push(self.efficiency.resource_generation);
        out.push(self.efficiency.factory_production);
        out.push(self.efficiency.factory_cost);
        out.push(self.efficiency.factory_operation);
        out.push(self.efficiency.mine_production);
        out.push(self.efficiency.mine_cost);
        out.push(self.efficiency.mine_operation);
        out.push(u8::from(self.cheap_factories));

        for cost in self.research.costs {
            out.push(cost.value());
        }
        out.push(u8::from(self.research.tech3));
        out.push(self.leftover.value());
    }
}

impl RaceSpec {
    pub(crate) fn decode_canonical(
        reader: &mut CanonicalReader<'_>,
    ) -> Result<Self, StateDecodeError> {
        let name = reader.read_string("race.name")?;
        let plural_name = reader.read_string("race.plural_name")?;
        let emblem = reader.read_u8("race.emblem")?;
        let password = reader.read_string("race.password")?;

        let primary_code = reader.read_u8("race.primary")?;
        let primary = PrimaryTrait::from_attribute_index(primary_code).ok_or_else(|| {
            reader.error(StateDecodeErrorKind::InvalidTag {
                field: "race.primary",
                value: primary_code,
            })
        })?;

        let lesser_bytes = reader.read_exact("race.lesser", 2)?;
        let lesser_bits = u16::from_le_bytes([lesser_bytes[0], lesser_bytes[1]]);
        let mut lesser = std::collections::BTreeSet::new();
        for bit in 0..14_u8 {
            if lesser_bits & (1_u16 << bit) == 0 {
                continue;
            }
            let trait_value = LesserTrait::from_bit_index(bit).ok_or_else(|| {
                reader.error(StateDecodeErrorKind::InvalidTag {
                    field: "race.lesser",
                    value: bit,
                })
            })?;
            lesser.insert(trait_value);
        }
        if lesser_bits >> 14 != 0 {
            return Err(reader.error(StateDecodeErrorKind::InvalidTag {
                field: "race.lesser",
                value: 14,
            }));
        }

        let growth_rate = reader.read_u8("race.growth_rate")?;

        let mut habitability = [HabitabilityRange::new(0, 100); 3];
        for band in &mut habitability {
            band.min = reader.read_u8("race.habitability.min")?;
            band.max = reader.read_u8("race.habitability.max")?;
            band.immune = reader.read_bool("race.habitability.immune")?;
        }

        let efficiency = Efficiency {
            resource_generation: reader.read_u8("race.efficiency.resource_generation")?,
            factory_production: reader.read_u8("race.efficiency.factory_production")?,
            factory_cost: reader.read_u8("race.efficiency.factory_cost")?,
            factory_operation: reader.read_u8("race.efficiency.factory_operation")?,
            mine_production: reader.read_u8("race.efficiency.mine_production")?,
            mine_cost: reader.read_u8("race.efficiency.mine_cost")?,
            mine_operation: reader.read_u8("race.efficiency.mine_operation")?,
        };

        let cheap_factories = reader.read_bool("race.cheap_factories")?;

        let mut costs = [ResearchCost::Standard; 6];
        for cost in &mut costs {
            let code = reader.read_u8("race.research.cost")?;
            *cost = ResearchCost::from_value(code).ok_or_else(|| {
                reader.error(StateDecodeErrorKind::InvalidTag {
                    field: "race.research.cost",
                    value: code,
                })
            })?;
        }
        let tech3 = reader.read_bool("race.research.tech3")?;

        let leftover_code = reader.read_u8("race.leftover")?;
        let leftover = LeftoverAllocation::from_value(leftover_code).ok_or_else(|| {
            reader.error(StateDecodeErrorKind::InvalidTag {
                field: "race.leftover",
                value: leftover_code,
            })
        })?;

        Ok(Self {
            name,
            plural_name,
            emblem,
            password,
            primary,
            lesser,
            growth_rate,
            habitability,
            efficiency,
            cheap_factories,
            research: ResearchSettings { costs, tech3 },
            leftover,
        })
    }
}

/// Returns the combined 24-row trait-to-effect checklist.
pub fn trait_effect_coverage() -> impl Iterator<Item = TraitEffectCoverage> {
    traits::PRIMARY_EFFECT_COVERAGE
        .into_iter()
        .chain(traits::LESSER_EFFECT_COVERAGE)
}

#[cfg(test)]
mod tests {
    use super::{LesserTrait, PrimaryTrait, RaceSpec, trait_effect_coverage};
    use crate::model::{CanonicalEncode, CanonicalReader};

    #[test]
    fn coverage_checklist_contains_all_24_traits() {
        let ids: Vec<_> = trait_effect_coverage().map(|row| row.trait_id).collect();
        assert_eq!(ids.len(), 24);
        let unique: std::collections::BTreeSet<_> = ids.iter().copied().collect();
        assert_eq!(unique.len(), 24);
        for trait_value in PrimaryTrait::ALL {
            assert!(unique.contains(trait_value.id()));
        }
        for trait_value in LesserTrait::ALL {
            assert!(unique.contains(trait_value.id()));
        }
    }

    #[test]
    fn race_spec_round_trips_through_canonical_bytes() {
        let mut spec = RaceSpec::humanoids_anchor();
        spec.lesser.insert(LesserTrait::TotalTerraforming);
        spec.lesser.insert(LesserTrait::UltimateRecycling);
        spec.cheap_factories = true;
        spec.research.tech3 = true;

        let mut bytes = Vec::new();
        spec.encode_canonical(&mut bytes);
        let mut reader = CanonicalReader::new(&bytes);
        let decoded = RaceSpec::decode_canonical(&mut reader).expect("race decodes");
        assert!(reader.is_finished());
        assert_eq!(decoded, spec);
    }
}
