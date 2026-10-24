//! Advantage-point evaluation.
//!
//! The structure and constants follow `CAdvantagePoints()` in
//! `stars-decompile/race.c`, with the two decompilation defects corrected as
//! documented in `docs/rules/advantage-points.md`. The accumulator uses
//! third-points and is divided by three exactly once at the end, truncating
//! toward zero like C integer division and Rust `/`.

use crate::race::habitability::HabitabilityModel;
use crate::race::spec::{RaceSpec, ResearchCost};
use crate::race::traits::{LesserTrait, PrimaryTrait};

/// Rule identifiers for the point-calculation terms.
///
/// The umbrella evidence rule is [`COST`]. These finer identifiers let tests
/// and the builder cite one formula term without inventing a new evidence
/// record for every arithmetic step.
pub mod rule_id {
    /// Umbrella rule for the complete recovered formula.
    pub const COST: &str = "RULE-RACE-POINTS-COST-001";
    /// Starting accumulator.
    pub const START: &str = "RULE-RACE-POINTS-START-001";
    /// Growth bonus and spread remapping.
    pub const GROWTH: &str = "RULE-RACE-POINTS-GROWTH-001";
    /// Innate habitability term.
    pub const HABITABILITY: &str = "RULE-RACE-POINTS-HABITABILITY-001";
    /// Ideal-point displacement and multiple-immunity penalty.
    pub const ENVIRONMENT: &str = "RULE-RACE-POINTS-ENVIRONMENT-001";
    /// Factory/growth/immunity interaction.
    pub const FACTORY_GROWTH: &str = "RULE-RACE-POINTS-FACTORY-GROWTH-001";
    /// Resource-generation curve.
    pub const RESOURCE: &str = "RULE-RACE-POINTS-RESOURCE-001";
    /// Factory efficiency block.
    pub const FACTORY: &str = "RULE-RACE-POINTS-FACTORY-001";
    /// Mine efficiency block.
    pub const MINE: &str = "RULE-RACE-POINTS-MINE-001";
    /// Alternate Reality replacement for both efficiency blocks.
    pub const ALTERNATE_REALITY: &str = "RULE-RACE-POINTS-AR-001";
    /// Primary trait cost.
    pub const PRIMARY: &str = "RULE-RACE-POINTS-PRIMARY-001";
    /// Lesser trait costs.
    pub const LESSER: &str = "RULE-RACE-POINTS-LESSER-001";
    /// Lesser-trait count and mixture penalties.
    pub const LESSER_PENALTY: &str = "RULE-RACE-POINTS-LESSER-PENALTY-001";
    /// No Advanced Scanners primary-trait adjustment.
    pub const NO_ADVANCED_SCANNERS: &str = "RULE-RACE-POINTS-NAS-001";
    /// Research-cost net curve.
    pub const RESEARCH: &str = "RULE-RACE-POINTS-RESEARCH-001";
    /// All-fields starting-technology checkbox.
    pub const TECH3: &str = "RULE-RACE-POINTS-TECH3-001";
    /// Alternate Reality research adjustment.
    pub const ALTERNATE_REALITY_RESEARCH: &str = "RULE-RACE-POINTS-AR-RESEARCH-001";
    /// Final third-point division.
    pub const FINAL: &str = "RULE-RACE-POINTS-FINAL-001";
}

const STARTING_RAW: i64 = 550 * 3;
const RESEARCH_DISADVANTAGE_POINTS: [i64; 6] = [150, 330, 540, 780, 1050, 1380];

/// One named contribution to the raw advantage-point accumulator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointTerm {
    /// Stable machine-readable key.
    pub key: &'static str,
    /// Display label.
    pub label: &'static str,
    /// Rule identifier for this formula term.
    pub rule_id: &'static str,
    /// Signed raw contribution in third-points.
    pub raw: i64,
}

/// A complete advantage-point evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceEvaluation {
    /// Contributions in application order. Their sum equals [`Self::raw`].
    pub terms: Vec<PointTerm>,
    /// Raw third-point accumulator.
    pub raw: i64,
    /// Displayed points: `raw / 3`, truncating toward zero.
    pub displayed: i32,
    /// Growth-spread factor after remapping.
    pub growth_spread: i32,
    /// Raw innate-habitability estimate, before step-2 scaling.
    pub innate_habitability: i32,
    /// Number of immune environment axes.
    pub immune_axes: u8,
}

impl RaceEvaluation {
    /// Finds a term by stable key.
    #[must_use]
    pub fn term(&self, key: &str) -> Option<&PointTerm> {
        self.terms.iter().find(|term| term.key == key)
    }

    /// Raw contribution of a term, or zero when it was not applied.
    #[must_use]
    pub fn term_raw(&self, key: &str) -> i64 {
        self.term(key).map_or(0, |term| term.raw)
    }

    /// Whether the displayed balance passes the non-negative gate.
    #[must_use]
    pub const fn is_affordable(&self) -> bool {
        self.displayed >= 0
    }
}

/// Evaluates a complete race with an injected habitability model.
#[must_use]
pub fn evaluate_race(spec: &RaceSpec, habitability: &dyn HabitabilityModel) -> RaceEvaluation {
    let mut terms = Vec::new();
    let mut raw = STARTING_RAW;
    terms.push(PointTerm {
        key: "starting",
        label: "Starting balance",
        rule_id: rule_id::START,
        raw,
    });

    // The decompilation clamps growth to 1..=20 and treats out-of-range input
    // as tampering. Validation rejects it before this point; clamp anyway so a
    // direct call cannot index an impossible remap.
    let growth = i64::from(spec.growth_rate.clamp(1, 20));

    let growth_bonus = match growth {
        1 => 5 * 4200,
        2 => 4 * 4200,
        3 => 3 * 4200,
        4 => 2 * 4200,
        5 => 4200,
        6 => 3600,
        7 => 2250,
        8 => 600,
        9 => 225,
        _ => 0,
    };
    raw += growth_bonus;
    terms.push(PointTerm {
        key: "growth_bonus",
        label: "Growth-rate bonus",
        rule_id: rule_id::GROWTH,
        raw: growth_bonus,
    });

    let growth_spread = match growth {
        1..=5 => growth,
        6..=13 => 5 + 2 * (growth - 5),
        14..=19 => 21 + 3 * (growth - 13),
        _ => 45,
    };

    let innate = i64::from(habitability.innate_habitability(spec));
    let innate_scaled = innate / 2000 * growth_spread / 24;
    raw -= innate_scaled;
    terms.push(PointTerm {
        key: "innate_habitability",
        label: "Innate habitability",
        rule_id: rule_id::HABITABILITY,
        raw: -innate_scaled,
    });

    let mut environmental = 0_i64;
    let mut immune_axes = 0_u8;
    for band in &spec.habitability {
        let ideal = i32::from(band.ideal());
        if ideal >= 0 {
            environmental += i64::from((ideal - 50).abs()) * 4;
        } else {
            immune_axes += 1;
        }
    }
    raw += environmental;
    terms.push(PointTerm {
        key: "ideal_displacement",
        label: "Ideal-point displacement",
        rule_id: rule_id::ENVIRONMENT,
        raw: environmental,
    });

    let immunity_penalty = if immune_axes > 1 { -150 } else { 0 };
    raw += immunity_penalty;
    terms.push(PointTerm {
        key: "immunity_penalty",
        label: "Multiple-immunity penalty",
        rule_id: rule_id::ENVIRONMENT,
        raw: immunity_penalty,
    });

    let factory_growth = factory_growth_interaction(spec, growth, immune_axes);
    raw += factory_growth;
    terms.push(PointTerm {
        key: "factory_growth_interaction",
        label: "Factory/growth interaction",
        rule_id: rule_id::FACTORY_GROWTH,
        raw: factory_growth,
    });

    let resource_generation = resource_generation_term(spec.efficiency.resource_generation);
    raw += resource_generation;
    terms.push(PointTerm {
        key: "resource_generation",
        label: "Resource generation",
        rule_id: rule_id::RESOURCE,
        raw: resource_generation,
    });

    if spec.primary.is_alternate_reality() {
        raw += 210;
        terms.push(PointTerm {
            key: "alternate_reality_block",
            label: "Alternate Reality efficiency block",
            rule_id: rule_id::ALTERNATE_REALITY,
            raw: 210,
        });
    } else {
        let factory = factory_block(spec, spec.cheap_factories);
        raw += factory;
        terms.push(PointTerm {
            key: "factory_block",
            label: "Factory efficiency",
            rule_id: rule_id::FACTORY,
            raw: factory,
        });

        let mine = mine_block(spec);
        raw += mine;
        terms.push(PointTerm {
            key: "mine_block",
            label: "Mine efficiency",
            rule_id: rule_id::MINE,
            raw: mine,
        });
    }

    let primary = -i64::from(spec.primary.point_cost_raw());
    raw += primary;
    terms.push(PointTerm {
        key: "primary_trait",
        label: spec.primary.name(),
        rule_id: rule_id::PRIMARY,
        raw: primary,
    });

    let mut cost_count = 0_i64;
    let mut gain_count = 0_i64;
    for trait_value in LesserTrait::ALL {
        if !spec.has_lesser(trait_value) {
            continue;
        }
        let cost = trait_value.cost();
        let value = i64::from(cost.raw);
        raw += value;
        if cost.raw < 0 {
            cost_count += 1;
        } else {
            gain_count += 1;
        }
        terms.push(PointTerm {
            key: trait_value.id(),
            label: trait_value.name(),
            rule_id: rule_id::LESSER,
            raw: value,
        });
    }

    let total_traits = cost_count + gain_count;
    let count_penalty = if total_traits > 4 {
        -((total_traits * 10) * (total_traits - 4))
    } else {
        0
    };
    raw += count_penalty;
    terms.push(PointTerm {
        key: "lesser_count_penalty",
        label: "Lesser-trait count penalty",
        rule_id: rule_id::LESSER_PENALTY,
        raw: count_penalty,
    });

    let mixture = gain_count - cost_count;
    let mixture_penalty = if mixture > 3 {
        -60 * (mixture - 3)
    } else if mixture < -3 {
        -40 * ((-mixture) - 3)
    } else {
        0
    };
    raw += mixture_penalty;
    terms.push(PointTerm {
        key: "lesser_mixture_penalty",
        label: "Lesser-trait mixture penalty",
        rule_id: rule_id::LESSER_PENALTY,
        raw: mixture_penalty,
    });

    let no_advanced_scanners = if spec.has_lesser(LesserTrait::NoAdvancedScanners) {
        match spec.primary {
            PrimaryTrait::PacketPhysics => -280,
            PrimaryTrait::SuperStealth => -200,
            PrimaryTrait::JackOfAllTrades => -40,
            _ => 0,
        }
    } else {
        0
    };
    raw += no_advanced_scanners;
    terms.push(PointTerm {
        key: "no_advanced_scanners",
        label: "No Advanced Scanners adjustment",
        rule_id: rule_id::NO_ADVANCED_SCANNERS,
        raw: no_advanced_scanners,
    });

    let research = research_cost_term(spec);
    raw += research;
    terms.push(PointTerm {
        key: "research_cost",
        label: "Research cost",
        rule_id: rule_id::RESEARCH,
        raw: research,
    });

    let tech3 = if spec.research.tech3 { -180 } else { 0 };
    raw += tech3;
    terms.push(PointTerm {
        key: "tech3",
        label: "All fields start at tech 3",
        rule_id: rule_id::TECH3,
        raw: tech3,
    });

    let alternate_reality_research =
        if spec.primary.is_alternate_reality() && spec.research.costs[0] == ResearchCost::Extra {
            -100
        } else {
            0
        };
    raw += alternate_reality_research;
    terms.push(PointTerm {
        key: "alternate_reality_research",
        label: "Alternate Reality research adjustment",
        rule_id: rule_id::ALTERNATE_REALITY_RESEARCH,
        raw: alternate_reality_research,
    });

    debug_assert_eq!(raw, terms.iter().map(|term| term.raw).sum::<i64>());

    #[allow(clippy::cast_possible_truncation)]
    let displayed = (raw / 3) as i32;

    RaceEvaluation {
        terms,
        raw,
        displayed,
        #[allow(clippy::cast_possible_truncation)]
        growth_spread: growth_spread as i32,
        innate_habitability: i32::try_from(innate).unwrap_or(i32::MAX),
        immune_axes,
    }
}

fn factory_growth_interaction(spec: &RaceSpec, growth: i64, immune_axes: u8) -> i64 {
    let operate = i64::from(spec.efficiency.factory_operation);
    let produce = i64::from(spec.efficiency.factory_production);
    if operate <= 10 && produce <= 10 {
        return 0;
    }

    let operate = (operate - 9).max(1);
    let mut produce = (produce - 9).max(1);
    if spec.primary == PrimaryTrait::HyperExpansion {
        produce *= 3;
    } else {
        produce *= 2;
    }
    let denominator = if immune_axes >= 2 { 2 } else { 9 };
    -operate * produce * growth / denominator
}

fn resource_generation_term(resource_generation: u8) -> i64 {
    let generation = i64::from(resource_generation.min(25));
    match generation {
        0..=7 => -2400,
        8 => -1260,
        9 => -600,
        10 => 0,
        _ => 120 * (generation - 10),
    }
}

fn factory_block(spec: &RaceSpec, cheap_factories: bool) -> i64 {
    let a = 10 - i64::from(spec.efficiency.factory_production);
    let b = 10 - i64::from(spec.efficiency.factory_cost);
    let c = 10 - i64::from(spec.efficiency.factory_operation);

    let mut value = if a > 0 { a * 100 } else { a * 121 };
    if b < 0 {
        value -= b * 55;
    } else {
        value -= b * b * 60;
    }
    value += if c > 0 { c * 40 } else { c * 35 };

    if value > 700 {
        value = 700 + (value - 700) / 3;
    }

    if c <= -7 {
        if c >= -11 {
            value -= 30 * (-6 - c);
        } else if c >= -14 {
            value -= 75 * 3 + 45 * (-12 - c);
        } else {
            value -= 120 * 3;
        }
    }

    if a <= -3 {
        value -= (-2 - a) * 20 * 3;
    }

    if cheap_factories {
        value -= 175;
    }
    value
}

fn mine_block(spec: &RaceSpec) -> i64 {
    let a = 10 - i64::from(spec.efficiency.mine_production);
    let b = 3 - i64::from(spec.efficiency.mine_cost);
    let c = 10 - i64::from(spec.efficiency.mine_operation);

    let mut value = if a > 0 { a * 100 } else { a * 169 };
    if b <= 0 {
        value -= -80 + b * 65;
    } else {
        value -= 360;
    }
    value += if c > 0 { c * 40 } else { c * 35 };
    value
}

fn research_cost_term(spec: &RaceSpec) -> i64 {
    let net: i64 = spec
        .research
        .costs
        .iter()
        .map(|cost| i64::from(cost.value()) - 1)
        .sum();

    if net > 0 {
        let mut value = -(net * net * 130);
        if net == 6 {
            value += (36 - 25) * 130;
        } else if net == 5 {
            value += (25 - 21) * 130;
        }
        value
    } else if net < 0 {
        let count = usize::try_from(-net).expect("net research count fits usize");
        let mut value = RESEARCH_DISADVANTAGE_POINTS[count - 1];
        if count > 4 && spec.efficiency.resource_generation < 10 {
            value -= 190;
        }
        value
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{RaceEvaluation, evaluate_race, rule_id};
    use crate::race::habitability::FixedHabitability;
    use crate::race::spec::{RaceSpec, ResearchCost};
    use crate::race::traits::{LesserTrait, PrimaryTrait};

    fn spec() -> RaceSpec {
        RaceSpec::humanoids_anchor()
    }

    fn evaluate(spec: &RaceSpec) -> RaceEvaluation {
        evaluate_race(spec, &FixedHabitability(0))
    }

    #[test]
    fn terms_sum_to_the_raw_accumulator() {
        let mut race = spec();
        race.lesser.insert(LesserTrait::TotalTerraforming);
        race.lesser.insert(LesserTrait::MineralAlchemy);
        let evaluation = evaluate(&race);
        assert_eq!(
            evaluation.raw,
            evaluation.terms.iter().map(|term| term.raw).sum::<i64>()
        );
        assert_eq!(evaluation.displayed, (evaluation.raw / 3) as i32);
    }

    #[test]
    fn every_term_has_a_non_empty_rule_id() {
        let evaluation = evaluate(&spec());
        for term in &evaluation.terms {
            assert!(!term.rule_id.is_empty(), "{}", term.key);
            assert!(!term.label.is_empty(), "{}", term.key);
        }
        assert!(evaluation.term("factory_block").is_some());
        assert!(evaluation.term("mine_block").is_some());
    }

    #[test]
    fn growth_bonus_breakpoints_match_the_recovered_table() {
        for (growth, expected) in [
            (1_u8, 5 * 4200),
            (2, 4 * 4200),
            (3, 3 * 4200),
            (4, 2 * 4200),
            (5, 4200),
            (6, 3600),
            (7, 2250),
            (8, 600),
            (9, 225),
            (10, 0),
        ] {
            let mut race = spec();
            race.growth_rate = growth;
            assert_eq!(
                evaluate(&race).term_raw("growth_bonus"),
                expected,
                "growth {growth}"
            );
        }
    }

    #[test]
    fn growth_spread_remap_includes_the_19_to_20_discontinuity() {
        for (growth, spread) in [
            (1_u8, 1),
            (5, 5),
            (6, 7),
            (13, 21),
            (14, 24),
            (19, 39),
            (20, 45),
        ] {
            let mut race = spec();
            race.growth_rate = growth;
            assert_eq!(evaluate(&race).growth_spread, spread, "growth {growth}");
        }
    }

    #[test]
    fn resource_generation_curve_matches_the_recovered_breakpoints() {
        for (value, expected) in [
            (7_u8, -2400),
            (8, -1260),
            (9, -600),
            (10, 0),
            (11, 120),
            (25, 1800),
        ] {
            let mut race = spec();
            race.efficiency.resource_generation = value;
            assert_eq!(
                evaluate(&race).term_raw("resource_generation"),
                expected,
                "resource generation {value}"
            );
        }
    }

    #[test]
    fn factory_sweep_matches_minus_60_zero_plus_55() {
        for (cost, expected) in [(9_u8, -60), (10, 0), (11, 55)] {
            let mut race = spec();
            race.efficiency.factory_cost = cost;
            assert_eq!(
                evaluate(&race).term_raw("factory_block"),
                expected,
                "factory cost {cost}"
            );
        }
    }

    #[test]
    fn mine_sweep_matches_minus_360_plus_80_plus_535() {
        for (cost, expected) in [(2_u8, -360), (3, 80), (10, 535)] {
            let mut race = spec();
            race.efficiency.mine_cost = cost;
            assert_eq!(
                evaluate(&race).term_raw("mine_block"),
                expected,
                "mine cost {cost}"
            );
        }
    }

    #[test]
    fn research_curve_matches_net_choices() {
        for (net, expected) in [
            (-6_i64, 1380),
            (-5, 1050),
            (-4, 780),
            (-3, 540),
            (-2, 330),
            (-1, 150),
            (0, 0),
            (1, -130),
            (2, -520),
            (3, -1170),
            (4, -2080),
            (5, -2730),
            (6, -3250),
        ] {
            let mut race = spec();
            for (index, cost) in race.research.costs.iter_mut().enumerate() {
                let index_value = i64::try_from(index).expect("index");
                *cost = if net > 0 && index_value < net {
                    ResearchCost::Extra
                } else if net < 0 && index_value < -net {
                    ResearchCost::Cheaper
                } else {
                    ResearchCost::Standard
                };
            }
            assert_eq!(
                evaluate(&race).term_raw("research_cost"),
                expected,
                "net research {net}"
            );
        }
    }

    #[test]
    fn lesser_trait_count_and_mixture_penalties_are_applied() {
        let four = [
            LesserTrait::ImprovedFuelEfficiency,
            LesserTrait::TotalTerraforming,
            LesserTrait::AdvancedRemoteMining,
            LesserTrait::ImprovedStarbases,
        ];
        let mut race = spec();
        race.lesser.extend(four);
        assert_eq!(evaluate(&race).term_raw("lesser_count_penalty"), 0);

        race.lesser.insert(LesserTrait::GeneralizedResearch);
        assert_eq!(
            evaluate(&race).term_raw("lesser_count_penalty"),
            -50,
            "five selected traits"
        );

        race.lesser.insert(LesserTrait::UltimateRecycling);
        assert_eq!(
            evaluate(&race).term_raw("lesser_count_penalty"),
            -120,
            "six selected traits"
        );
    }

    #[test]
    fn alternate_reality_replaces_both_efficiency_blocks() {
        let mut race = spec();
        race.primary = PrimaryTrait::AlternateReality;
        let evaluation = evaluate(&race);
        assert_eq!(evaluation.term_raw("alternate_reality_block"), 210);
        assert!(evaluation.term("factory_block").is_none());
        assert!(evaluation.term("mine_block").is_none());
    }

    #[test]
    fn no_advanced_scanners_pairing_follows_the_primary_trait() {
        for (primary, expected) in [
            (PrimaryTrait::PacketPhysics, -280),
            (PrimaryTrait::SuperStealth, -200),
            (PrimaryTrait::JackOfAllTrades, -40),
            (PrimaryTrait::WarMonger, 0),
        ] {
            let mut race = spec();
            race.primary = primary;
            race.lesser.insert(LesserTrait::NoAdvancedScanners);
            assert_eq!(
                evaluate(&race).term_raw("no_advanced_scanners"),
                expected,
                "{primary}"
            );
        }
    }

    #[test]
    fn cheap_factories_has_its_own_option_adjustment() {
        let mut race = spec();
        let without = evaluate(&race).term_raw("factory_block");
        race.cheap_factories = true;
        let with = evaluate(&race).term_raw("factory_block");
        assert_eq!(with, without - 175);
    }

    #[test]
    fn alternate_reality_uses_the_resource_generation_coefficient_only() {
        let mut race = spec();
        race.primary = PrimaryTrait::AlternateReality;
        race.efficiency.resource_generation = 7;
        race.efficiency.factory_production = 15;
        race.efficiency.mine_cost = 2;
        let evaluation = evaluate(&race);

        assert_eq!(evaluation.term_raw("resource_generation"), -2400);
        assert_eq!(evaluation.term_raw("alternate_reality_block"), 210);
        assert!(evaluation.term("factory_block").is_none());
        assert!(evaluation.term("mine_block").is_none());
    }

    #[test]
    fn alternate_reality_research_adjustment_uses_the_first_field_only() {
        let mut race = spec();
        race.primary = PrimaryTrait::AlternateReality;
        race.research.costs[0] = ResearchCost::Extra;
        assert_eq!(evaluate(&race).term_raw("alternate_reality_research"), -100);

        let mut other_field = spec();
        other_field.primary = PrimaryTrait::AlternateReality;
        other_field.research.costs[1] = ResearchCost::Extra;
        assert_eq!(
            evaluate(&other_field).term_raw("alternate_reality_research"),
            0
        );
    }

    #[test]
    fn tech3_checkbox_costs_180() {
        let mut race = spec();
        assert_eq!(evaluate(&race).term_raw("tech3"), 0);
        race.research.tech3 = true;
        assert_eq!(evaluate(&race).term_raw("tech3"), -180);
    }

    #[test]
    fn alternate_reality_uses_the_same_efficiency_defaults_but_adds_210() {
        let standard = evaluate(&spec());
        let mut alternate = spec();
        alternate.primary = PrimaryTrait::AlternateReality;
        let alternate = evaluate(&alternate);
        assert_eq!(
            alternate.raw,
            standard.raw - (standard.term_raw("factory_block") + standard.term_raw("mine_block"))
                + 210
                - (PrimaryTrait::AlternateReality.point_cost_raw()
                    - PrimaryTrait::JackOfAllTrades.point_cost_raw()) as i64
        );
    }

    #[test]
    fn fixed_habitability_keeps_the_innate_term_isolated() {
        let evaluation = evaluate(&spec());
        assert_eq!(evaluation.term_raw("innate_habitability"), 0);
        assert_eq!(evaluation.innate_habitability, 0);
    }

    #[test]
    fn umbrella_rule_is_the_documented_cost_rule() {
        assert_eq!(rule_id::COST, "RULE-RACE-POINTS-COST-001");
    }
}
