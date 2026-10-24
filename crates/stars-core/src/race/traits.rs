//! Typed primary and lesser racial traits.
//!
//! The trait order, point values and attribute indices are transcribed from
//! `stars-decompile/race.c` and `stars-decompile/enums.h`, which were recovered
//! from NB09 CodeView symbols appended to `stars26jrc3.exe`. None of these
//! values is an observed result from `stars26i.exe`; see
//! `docs/rules/advantage-points.md` for the evidence classification.
//!
//! All ten primary and all fourteen lesser traits are selectable for a custom
//! human race. Primary traits are capabilities, not preset races. Selecting a
//! trait here does not claim that its runtime effect exists: the coverage table
//! at the bottom of this file records the later PR that owns each effect.

use std::fmt;

/// Evidence strength for a point value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostEvidence {
    /// Transcribed from the recovered `CAdvantagePoints()` table.
    StaticallyDerived,
    /// The only unexplained adjacent table value; not identified by the
    /// decompiled enum.
    Inferred,
}

impl CostEvidence {
    /// Human-readable label used by the builder.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StaticallyDerived => "statically derived",
            Self::Inferred => "inferred",
        }
    }
}

/// A trait point value and the strength of its evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraitCost {
    /// Internal third-point value added to the accumulator.
    pub raw: i32,
    /// How the value was obtained.
    pub evidence: CostEvidence,
}

/// One of the ten primary racial traits.
///
/// The declaration order matches `RaceAttribute` in `enums.h` and the
/// `rgRacePrimaryTrait` table. `JackOfAllTrades` is the `raNone` entry: it is a
/// real primary trait, not the absence of a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimaryTrait {
    /// Hyper-Expansion (`PRT.HE`).
    HyperExpansion,
    /// Super-Stealth (`PRT.SS`).
    SuperStealth,
    /// War Monger (`PRT.WM`).
    WarMonger,
    /// Claim Adjuster (`PRT.CA`).
    ClaimAdjuster,
    /// Inner-Strength (`PRT.IS`).
    InnerStrength,
    /// Space Demolition (`PRT.SD`).
    SpaceDemolition,
    /// Packet Physics (`PRT.PP`).
    PacketPhysics,
    /// Interstellar Traveler (`PRT.IT`).
    InterstellarTraveler,
    /// Alternate Reality (`PRT.AR`).
    AlternateReality,
    /// Jack of All Trades (`PRT.JOAT`).
    JackOfAllTrades,
}

impl PrimaryTrait {
    /// All primary traits in `RaceAttribute` order.
    pub const ALL: [Self; 10] = [
        Self::HyperExpansion,
        Self::SuperStealth,
        Self::WarMonger,
        Self::ClaimAdjuster,
        Self::InnerStrength,
        Self::SpaceDemolition,
        Self::PacketPhysics,
        Self::InterstellarTraveler,
        Self::AlternateReality,
        Self::JackOfAllTrades,
    ];

    /// Stable coverage-matrix identifier, such as `PRT.HE`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::HyperExpansion => "PRT.HE",
            Self::SuperStealth => "PRT.SS",
            Self::WarMonger => "PRT.WM",
            Self::ClaimAdjuster => "PRT.CA",
            Self::InnerStrength => "PRT.IS",
            Self::SpaceDemolition => "PRT.SD",
            Self::PacketPhysics => "PRT.PP",
            Self::InterstellarTraveler => "PRT.IT",
            Self::AlternateReality => "PRT.AR",
            Self::JackOfAllTrades => "PRT.JOAT",
        }
    }

    /// Display name used by the builder.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::HyperExpansion => "Hyper-Expansion",
            Self::SuperStealth => "Super-Stealth",
            Self::WarMonger => "War Monger",
            Self::ClaimAdjuster => "Claim Adjuster",
            Self::InnerStrength => "Inner-Strength",
            Self::SpaceDemolition => "Space Demolition",
            Self::PacketPhysics => "Packet Physics",
            Self::InterstellarTraveler => "Interstellar Traveler",
            Self::AlternateReality => "Alternate Reality",
            Self::JackOfAllTrades => "Jack of All Trades",
        }
    }

    /// Two-to-four letter abbreviation used in compact summaries.
    #[must_use]
    pub const fn abbreviation(self) -> &'static str {
        match self {
            Self::HyperExpansion => "HE",
            Self::SuperStealth => "SS",
            Self::WarMonger => "WM",
            Self::ClaimAdjuster => "CA",
            Self::InnerStrength => "IS",
            Self::SpaceDemolition => "SD",
            Self::PacketPhysics => "PP",
            Self::InterstellarTraveler => "IT",
            Self::AlternateReality => "AR",
            Self::JackOfAllTrades => "JOAT",
        }
    }

    /// Index in `RaceAttribute` and `rgRacePrimaryTrait`.
    #[must_use]
    pub const fn attribute_index(self) -> u8 {
        match self {
            Self::HyperExpansion => 0,
            Self::SuperStealth => 1,
            Self::WarMonger => 2,
            Self::ClaimAdjuster => 3,
            Self::InnerStrength => 4,
            Self::SpaceDemolition => 5,
            Self::PacketPhysics => 6,
            Self::InterstellarTraveler => 7,
            Self::AlternateReality => 8,
            Self::JackOfAllTrades => 9,
        }
    }

    /// Converts a persisted attribute index back to a trait.
    #[must_use]
    pub const fn from_attribute_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::HyperExpansion),
            1 => Some(Self::SuperStealth),
            2 => Some(Self::WarMonger),
            3 => Some(Self::ClaimAdjuster),
            4 => Some(Self::InnerStrength),
            5 => Some(Self::SpaceDemolition),
            6 => Some(Self::PacketPhysics),
            7 => Some(Self::InterstellarTraveler),
            8 => Some(Self::AlternateReality),
            9 => Some(Self::JackOfAllTrades),
            _ => None,
        }
    }

    /// Signed raw point value from `rgRacePrimaryTrait`.
    ///
    /// A positive entry is a cost subtracted from the accumulator; a negative
    /// entry is a gain added to it.
    #[must_use]
    pub const fn point_cost_raw(self) -> i32 {
        match self {
            Self::HyperExpansion => 40,
            Self::SuperStealth => 95,
            Self::WarMonger => 45,
            Self::ClaimAdjuster => 10,
            Self::InnerStrength => -100,
            Self::SpaceDemolition => -150,
            Self::PacketPhysics => 120,
            Self::InterstellarTraveler => 180,
            Self::AlternateReality => 90,
            Self::JackOfAllTrades => -66,
        }
    }

    /// Whether this is Alternate Reality, whose efficiency blocks have a
    /// special replacement in the point calculation.
    #[must_use]
    pub const fn is_alternate_reality(self) -> bool {
        matches!(self, Self::AlternateReality)
    }

    /// Whether the race can build planetary installations such as factories,
    /// mines and defenses.
    ///
    /// The help documents that Alternate Reality races cannot build planetary
    /// installations; their Step 5 efficiency page therefore exposes only the
    /// resource-generation coefficient.
    #[must_use]
    pub const fn has_planetary_installations(self) -> bool {
        !self.is_alternate_reality()
    }

    /// Later PRs that own the runtime effect.
    #[must_use]
    pub const fn effect_owner_prs(self) -> &'static [&'static str] {
        match self {
            Self::HyperExpansion => &["PR6", "PR8", "PR9", "PR10", "PR13"],
            Self::SuperStealth => &["PR6", "PR9", "PR10", "PR11", "PR12", "PR13"],
            Self::WarMonger => &["PR6", "PR9", "PR10", "PR12", "PR13"],
            Self::ClaimAdjuster => &["PR6", "PR8", "PR9", "PR13"],
            Self::InnerStrength => &["PR6", "PR9", "PR10", "PR12", "PR13"],
            Self::SpaceDemolition => &["PR6", "PR9", "PR10", "PR11", "PR12", "PR13"],
            Self::PacketPhysics => &["PR6", "PR8", "PR9", "PR10", "PR11", "PR12", "PR13"],
            Self::InterstellarTraveler => &["PR6", "PR9", "PR10", "PR11", "PR12", "PR13"],
            Self::AlternateReality => &["PR6", "PR8", "PR9", "PR10", "PR11", "PR13"],
            Self::JackOfAllTrades => &["PR6", "PR8", "PR9", "PR11", "PR13"],
        }
    }
}

impl fmt::Display for PrimaryTrait {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One of the fourteen lesser racial traits.
///
/// Declaration order follows the recovered `rgRaceAdvDisPts` indices. Bit 5 is
/// absent from the decompiled `RaceGrbit` enum; the unexplained `-240` at that
/// index is treated as Ultimate Recycling's price and labelled inferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LesserTrait {
    /// Improved Fuel Efficiency (`LRT.IFE`).
    ImprovedFuelEfficiency,
    /// Total Terraforming (`LRT.TT`).
    TotalTerraforming,
    /// Advanced Remote Mining (`LRT.ARM`).
    AdvancedRemoteMining,
    /// Improved Starbases (`LRT.ISB`).
    ImprovedStarbases,
    /// Generalized Research (`LRT.GR`).
    GeneralizedResearch,
    /// Ultimate Recycling (`LRT.UR`).
    UltimateRecycling,
    /// Mineral Alchemy (`LRT.MA`).
    MineralAlchemy,
    /// No Ramscoop Engines (`LRT.NRSE`).
    NoRamscoopEngines,
    /// Cheap Engines (`LRT.CE`).
    CheapEngines,
    /// Only Basic Remote Mining (`LRT.OBRM`).
    OnlyBasicRemoteMining,
    /// No Advanced Scanners (`LRT.NAS`).
    NoAdvancedScanners,
    /// Low Starting Population (`LRT.LSP`).
    LowStartingPopulation,
    /// Bleeding Edge Technology (`LRT.BET`).
    BleedingEdgeTechnology,
    /// Regenerating Shields (`LRT.RS`).
    RegeneratingShields,
}

impl LesserTrait {
    /// All lesser traits in recovered bit order.
    pub const ALL: [Self; 14] = [
        Self::ImprovedFuelEfficiency,
        Self::TotalTerraforming,
        Self::AdvancedRemoteMining,
        Self::ImprovedStarbases,
        Self::GeneralizedResearch,
        Self::UltimateRecycling,
        Self::MineralAlchemy,
        Self::NoRamscoopEngines,
        Self::CheapEngines,
        Self::OnlyBasicRemoteMining,
        Self::NoAdvancedScanners,
        Self::LowStartingPopulation,
        Self::BleedingEdgeTechnology,
        Self::RegeneratingShields,
    ];

    /// Stable coverage-matrix identifier, such as `LRT.TT`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ImprovedFuelEfficiency => "LRT.IFE",
            Self::TotalTerraforming => "LRT.TT",
            Self::AdvancedRemoteMining => "LRT.ARM",
            Self::ImprovedStarbases => "LRT.ISB",
            Self::GeneralizedResearch => "LRT.GR",
            Self::UltimateRecycling => "LRT.UR",
            Self::MineralAlchemy => "LRT.MA",
            Self::NoRamscoopEngines => "LRT.NRSE",
            Self::CheapEngines => "LRT.CE",
            Self::OnlyBasicRemoteMining => "LRT.OBRM",
            Self::NoAdvancedScanners => "LRT.NAS",
            Self::LowStartingPopulation => "LRT.LSP",
            Self::BleedingEdgeTechnology => "LRT.BET",
            Self::RegeneratingShields => "LRT.RS",
        }
    }

    /// Display name used by the builder.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ImprovedFuelEfficiency => "Improved Fuel Efficiency",
            Self::TotalTerraforming => "Total Terraforming",
            Self::AdvancedRemoteMining => "Advanced Remote Mining",
            Self::ImprovedStarbases => "Improved Starbases",
            Self::GeneralizedResearch => "Generalized Research",
            Self::UltimateRecycling => "Ultimate Recycling",
            Self::MineralAlchemy => "Mineral Alchemy",
            Self::NoRamscoopEngines => "No Ramscoop Engines",
            Self::CheapEngines => "Cheap Engines",
            Self::OnlyBasicRemoteMining => "Only Basic Remote Mining",
            Self::NoAdvancedScanners => "No Advanced Scanners",
            Self::LowStartingPopulation => "Low Starting Population",
            Self::BleedingEdgeTechnology => "Bleeding Edge Technology",
            Self::RegeneratingShields => "Regenerating Shields",
        }
    }

    /// Abbreviation used in compact summaries.
    #[must_use]
    pub const fn abbreviation(self) -> &'static str {
        match self {
            Self::ImprovedFuelEfficiency => "IFE",
            Self::TotalTerraforming => "TT",
            Self::AdvancedRemoteMining => "ARM",
            Self::ImprovedStarbases => "ISB",
            Self::GeneralizedResearch => "GR",
            Self::UltimateRecycling => "UR",
            Self::MineralAlchemy => "MA",
            Self::NoRamscoopEngines => "NRSE",
            Self::CheapEngines => "CE",
            Self::OnlyBasicRemoteMining => "OBRM",
            Self::NoAdvancedScanners => "NAS",
            Self::LowStartingPopulation => "LSP",
            Self::BleedingEdgeTechnology => "BET",
            Self::RegeneratingShields => "RS",
        }
    }

    /// Index in the recovered `rgRaceAdvDisPts` table.
    #[must_use]
    pub const fn bit_index(self) -> u8 {
        match self {
            Self::ImprovedFuelEfficiency => 0,
            Self::TotalTerraforming => 1,
            Self::AdvancedRemoteMining => 2,
            Self::ImprovedStarbases => 3,
            Self::GeneralizedResearch => 4,
            Self::UltimateRecycling => 5,
            Self::MineralAlchemy => 6,
            Self::NoRamscoopEngines => 7,
            Self::CheapEngines => 8,
            Self::OnlyBasicRemoteMining => 9,
            Self::NoAdvancedScanners => 10,
            Self::LowStartingPopulation => 11,
            Self::BleedingEdgeTechnology => 12,
            Self::RegeneratingShields => 13,
        }
    }

    /// Converts a persisted bit index back to a trait.
    #[must_use]
    pub const fn from_bit_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::ImprovedFuelEfficiency),
            1 => Some(Self::TotalTerraforming),
            2 => Some(Self::AdvancedRemoteMining),
            3 => Some(Self::ImprovedStarbases),
            4 => Some(Self::GeneralizedResearch),
            5 => Some(Self::UltimateRecycling),
            6 => Some(Self::MineralAlchemy),
            7 => Some(Self::NoRamscoopEngines),
            8 => Some(Self::CheapEngines),
            9 => Some(Self::OnlyBasicRemoteMining),
            10 => Some(Self::NoAdvancedScanners),
            11 => Some(Self::LowStartingPopulation),
            12 => Some(Self::BleedingEdgeTechnology),
            13 => Some(Self::RegeneratingShields),
            _ => None,
        }
    }

    /// Raw point value and evidence status.
    #[must_use]
    pub const fn cost(self) -> TraitCost {
        match self {
            Self::ImprovedFuelEfficiency => TraitCost {
                raw: -235,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::TotalTerraforming => TraitCost {
                raw: -25,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::AdvancedRemoteMining => TraitCost {
                raw: -159,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::ImprovedStarbases => TraitCost {
                raw: -201,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::GeneralizedResearch => TraitCost {
                raw: 40,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::UltimateRecycling => TraitCost {
                raw: -240,
                evidence: CostEvidence::Inferred,
            },
            Self::MineralAlchemy => TraitCost {
                raw: -155,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::NoRamscoopEngines => TraitCost {
                raw: 160,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::CheapEngines => TraitCost {
                raw: 240,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::OnlyBasicRemoteMining => TraitCost {
                raw: 255,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::NoAdvancedScanners => TraitCost {
                raw: 325,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::LowStartingPopulation => TraitCost {
                raw: 180,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::BleedingEdgeTechnology => TraitCost {
                raw: 70,
                evidence: CostEvidence::StaticallyDerived,
            },
            Self::RegeneratingShields => TraitCost {
                raw: 30,
                evidence: CostEvidence::StaticallyDerived,
            },
        }
    }

    /// Later PRs that own the runtime effect.
    #[must_use]
    pub const fn effect_owner_prs(self) -> &'static [&'static str] {
        match self {
            Self::ImprovedFuelEfficiency => &["PR6", "PR9", "PR10", "PR13"],
            Self::TotalTerraforming => &["PR6", "PR8", "PR13"],
            Self::AdvancedRemoteMining => &["PR6", "PR8", "PR9", "PR13"],
            Self::ImprovedStarbases => &["PR6", "PR9", "PR12", "PR13"],
            Self::GeneralizedResearch => &["PR6", "PR9", "PR13"],
            Self::UltimateRecycling => &["PR6", "PR9", "PR10", "PR13"],
            Self::MineralAlchemy => &["PR6", "PR8", "PR13"],
            Self::NoRamscoopEngines => &["PR6", "PR9", "PR10", "PR13"],
            Self::CheapEngines => &["PR6", "PR9", "PR10", "PR13"],
            Self::OnlyBasicRemoteMining => &["PR6", "PR8", "PR9", "PR13"],
            Self::NoAdvancedScanners => &["PR6", "PR9", "PR11", "PR13"],
            Self::LowStartingPopulation => &["PR6", "PR8", "PR13"],
            Self::BleedingEdgeTechnology => &["PR6", "PR9", "PR13"],
            Self::RegeneratingShields => &["PR6", "PR9", "PR12", "PR13"],
        }
    }
}

impl fmt::Display for LesserTrait {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One row of the trait-to-effect checklist.
///
/// PR5 can represent and price every row, but no runtime effect is claimed
/// until its owning PR lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraitEffectCoverage {
    /// Stable trait identifier.
    pub trait_id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Whether PR5 represents the selection.
    pub builder_supported: bool,
    /// Whether a runtime gameplay effect exists in this build.
    pub runtime_implemented: bool,
    /// Later PRs that own the runtime effect.
    pub owner_prs: &'static [&'static str],
}

/// Coverage checklist for the ten primary traits.
pub const PRIMARY_EFFECT_COVERAGE: [TraitEffectCoverage; 10] = [
    coverage(PrimaryTrait::HyperExpansion),
    coverage(PrimaryTrait::SuperStealth),
    coverage(PrimaryTrait::WarMonger),
    coverage(PrimaryTrait::ClaimAdjuster),
    coverage(PrimaryTrait::InnerStrength),
    coverage(PrimaryTrait::SpaceDemolition),
    coverage(PrimaryTrait::PacketPhysics),
    coverage(PrimaryTrait::InterstellarTraveler),
    coverage(PrimaryTrait::AlternateReality),
    coverage(PrimaryTrait::JackOfAllTrades),
];

/// Coverage checklist for the fourteen lesser traits.
pub const LESSER_EFFECT_COVERAGE: [TraitEffectCoverage; 14] = [
    lesser_coverage(LesserTrait::ImprovedFuelEfficiency),
    lesser_coverage(LesserTrait::TotalTerraforming),
    lesser_coverage(LesserTrait::AdvancedRemoteMining),
    lesser_coverage(LesserTrait::ImprovedStarbases),
    lesser_coverage(LesserTrait::GeneralizedResearch),
    lesser_coverage(LesserTrait::UltimateRecycling),
    lesser_coverage(LesserTrait::MineralAlchemy),
    lesser_coverage(LesserTrait::NoRamscoopEngines),
    lesser_coverage(LesserTrait::CheapEngines),
    lesser_coverage(LesserTrait::OnlyBasicRemoteMining),
    lesser_coverage(LesserTrait::NoAdvancedScanners),
    lesser_coverage(LesserTrait::LowStartingPopulation),
    lesser_coverage(LesserTrait::BleedingEdgeTechnology),
    lesser_coverage(LesserTrait::RegeneratingShields),
];

const fn coverage(trait_value: PrimaryTrait) -> TraitEffectCoverage {
    TraitEffectCoverage {
        trait_id: trait_value.id(),
        name: trait_value.name(),
        builder_supported: true,
        runtime_implemented: false,
        owner_prs: trait_value.effect_owner_prs(),
    }
}

const fn lesser_coverage(trait_value: LesserTrait) -> TraitEffectCoverage {
    TraitEffectCoverage {
        trait_id: trait_value.id(),
        name: trait_value.name(),
        builder_supported: true,
        runtime_implemented: false,
        owner_prs: trait_value.effect_owner_prs(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LESSER_EFFECT_COVERAGE, LesserTrait, PRIMARY_EFFECT_COVERAGE, PrimaryTrait, TraitCost,
    };

    #[test]
    fn all_primary_traits_have_unique_ids_and_indices() {
        let mut ids = std::collections::BTreeSet::new();
        for (index, trait_value) in PrimaryTrait::ALL.into_iter().enumerate() {
            assert!(
                ids.insert(trait_value.id()),
                "duplicate {}",
                trait_value.id()
            );
            assert_eq!(trait_value.attribute_index(), index as u8);
            assert_eq!(
                PrimaryTrait::from_attribute_index(index as u8),
                Some(trait_value)
            );
        }
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn all_lesser_traits_have_unique_ids_and_bit_indices() {
        let mut ids = std::collections::BTreeSet::new();
        for (index, trait_value) in LesserTrait::ALL.into_iter().enumerate() {
            assert!(
                ids.insert(trait_value.id()),
                "duplicate {}",
                trait_value.id()
            );
            assert_eq!(trait_value.bit_index(), index as u8);
            assert_eq!(LesserTrait::from_bit_index(index as u8), Some(trait_value));
        }
        assert_eq!(ids.len(), 14);
    }

    #[test]
    fn only_ultimate_recycling_has_an_inferred_price() {
        for trait_value in LesserTrait::ALL {
            let expected = match trait_value {
                LesserTrait::UltimateRecycling => super::CostEvidence::Inferred,
                _ => super::CostEvidence::StaticallyDerived,
            };
            assert_eq!(trait_value.cost().evidence, expected);
        }
        assert_eq!(
            LesserTrait::UltimateRecycling.cost(),
            TraitCost {
                raw: -240,
                evidence: super::CostEvidence::Inferred
            }
        );
    }

    #[test]
    fn only_alternate_reality_lacks_planetary_installations() {
        for trait_value in PrimaryTrait::ALL {
            assert_eq!(
                trait_value.has_planetary_installations(),
                trait_value != PrimaryTrait::AlternateReality,
                "{trait_value}"
            );
        }
    }

    #[test]
    fn every_trait_row_is_builder_supported_and_runtime_pending() {
        assert_eq!(PRIMARY_EFFECT_COVERAGE.len(), 10);
        assert_eq!(LESSER_EFFECT_COVERAGE.len(), 14);
        for row in PRIMARY_EFFECT_COVERAGE
            .into_iter()
            .chain(LESSER_EFFECT_COVERAGE)
        {
            assert!(row.builder_supported, "{}", row.trait_id);
            assert!(!row.runtime_implemented, "{}", row.trait_id);
            assert!(!row.owner_prs.is_empty(), "{}", row.trait_id);
        }
    }
}
