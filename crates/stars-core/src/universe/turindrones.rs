//! Built-in Turindrones race configurations.
//!
//! The supplied executable only proves that the name `Turindrones` exists. The
//! decompilation of the later 2.60j RC3 build contains the six computer-race
//! presets and their four proficiency records in `vrgplrComp`. Index 1 is
//! Turindrones; its four records map to AI levels 0..=3.
//!
//! This module reconstructs those race fields as a [`RaceSpec`]. It does not
//! claim the AI decision policy: planning and memory remain PR14 work. The
//! expert profile's `0x2000_0000` flag is the recovered Tech 3 flag
//! (`ibitRaceTech3`), and `0x8000_0000` is Cheap Factories.

use std::collections::BTreeSet;

use crate::race::{
    Efficiency, HabitabilityRange, LeftoverAllocation, LesserTrait, PrimaryTrait, RaceSpec,
    ResearchCost, ResearchSettings, validate_race_fields,
};
use crate::universe::settings::Difficulty;

/// Stable display name of the built-in opponent.
pub const TURINDRONES_NAME: &str = "Turindrones";

/// One recovered `vrgplrComp[1][level]` record.
#[derive(Clone, Copy, Debug)]
struct Profile {
    growth: u8,
    environment_min: [i16; 3],
    environment_max: [i16; 3],
    attributes: [u8; 15],
    flags: u32,
}

/// The four Turindrones proficiency records in AI-level order.
const PROFILES: [Profile; 4] = [
    Profile {
        growth: 14,
        environment_min: [27, 7, 35],
        environment_max: [89, 63, 95],
        attributes: [10, 9, 10, 9, 9, 5, 8, 0, 1, 0, 1, 1, 1, 0, 1],
        flags: 0x0000_2045,
    },
    Profile {
        growth: 14,
        environment_min: [32, 6, 26],
        environment_max: [92, 60, 96],
        attributes: [10, 10, 10, 10, 10, 5, 9, 0, 1, 1, 1, 1, 1, 1, 1],
        flags: 0x8000_2045,
    },
    Profile {
        growth: 14,
        environment_min: [31, 4, 30],
        environment_max: [95, 52, 94],
        attributes: [9, 11, 10, 10, 10, 5, 9, 0, 0, 1, 0, 1, 1, 1, 1],
        flags: 0x8000_2045,
    },
    Profile {
        growth: 15,
        environment_min: [31, 5, 0],
        environment_max: [93, 53, -1],
        attributes: [8, 15, 10, 25, 10, 5, 9, 0, 0, 0, 0, 0, 0, 0, 1],
        flags: 0xA000_2045,
    },
];

/// Returns the recovered Turindrones race for one difficulty level.
///
/// The returned race passes field validation. Its advantage-point balance is
/// not used to reject an AI slot because the original computer presets are not
/// custom-race purchases.
///
/// # Panics
///
/// Panics only if the compiled-in profile table is inconsistent, which is a
/// programming error rather than user input.
#[must_use]
pub fn turindrones_race(difficulty: Difficulty) -> RaceSpec {
    let profile = PROFILES[difficulty.stored_value() as usize];
    let primary = PrimaryTrait::from_attribute_index(profile.attributes[14])
        .unwrap_or(PrimaryTrait::SuperStealth);

    let mut lesser = BTreeSet::new();
    for trait_value in LesserTrait::ALL {
        if profile.flags & (1_u32 << trait_value.bit_index()) != 0 {
            lesser.insert(trait_value);
        }
    }

    let habitability = std::array::from_fn(|axis| {
        let min = u8::try_from(profile.environment_min[axis].max(0)).unwrap_or(0);
        if profile.environment_max[axis] < 0 {
            HabitabilityRange::immune_with_bounds(min, 100)
        } else {
            HabitabilityRange::new(
                min,
                u8::try_from(profile.environment_max[axis].max(0)).unwrap_or(100),
            )
        }
    });

    let mut costs = [ResearchCost::Standard; 6];
    for (index, value) in profile.attributes[8..14].iter().copied().enumerate() {
        costs[index] = ResearchCost::from_value(value).unwrap_or(ResearchCost::Standard);
    }

    let race = RaceSpec {
        name: TURINDRONES_NAME.to_owned(),
        plural_name: TURINDRONES_NAME.to_owned(),
        emblem: 0,
        password: String::new(),
        primary,
        lesser,
        growth_rate: profile.growth,
        habitability,
        efficiency: Efficiency {
            resource_generation: profile.attributes[0],
            factory_production: profile.attributes[1],
            factory_cost: profile.attributes[2],
            factory_operation: profile.attributes[3],
            mine_production: profile.attributes[4],
            mine_cost: profile.attributes[5],
            mine_operation: profile.attributes[6],
        },
        cheap_factories: profile.flags & (1_u32 << 31) != 0,
        research: ResearchSettings {
            costs,
            tech3: profile.flags & (1_u32 << 0x1d) != 0,
        },
        leftover: LeftoverAllocation::from_value(profile.attributes[7])
            .unwrap_or(LeftoverAllocation::SurfaceMinerals),
    };

    assert!(
        validate_race_fields(&race).is_empty(),
        "compiled-in Turindrones profile must be structurally valid"
    );
    race
}

#[cfg(test)]
mod tests {
    use super::{TURINDRONES_NAME, turindrones_race};
    use crate::race::{LesserTrait, PrimaryTrait, validate_race_fields};
    use crate::universe::settings::Difficulty;

    #[test]
    fn every_proficiency_profile_is_a_valid_super_stealth_race() {
        for difficulty in Difficulty::ALL {
            let race = turindrones_race(difficulty);
            assert_eq!(race.name, TURINDRONES_NAME);
            assert_eq!(race.primary, PrimaryTrait::SuperStealth);
            assert!(validate_race_fields(&race).is_empty(), "{difficulty:?}");
            assert!(race.has_lesser(LesserTrait::ImprovedFuelEfficiency));
            assert!(race.has_lesser(LesserTrait::AdvancedRemoteMining));
            assert!(race.has_lesser(LesserTrait::MineralAlchemy));
            assert!(race.has_lesser(LesserTrait::RegeneratingShields));
        }
    }

    #[test]
    fn expert_profile_recovers_the_immune_radiation_axis_and_cheap_factories() {
        let race = turindrones_race(Difficulty::Expert);
        assert!(race.habitability[2].immune);
        assert!(race.cheap_factories);
        assert!(race.research.tech3);
        assert!(
            race.research
                .costs
                .iter()
                .all(|cost| *cost == crate::race::ResearchCost::Extra)
        );
    }
}
