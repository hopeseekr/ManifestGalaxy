//! Innate-habitability sampling used by the advantage-point calculation.
//!
//! `LInnateRaceHabitability()` is recovered from `stars-decompile/race.c`. It
//! samples each environmental band at three terraforming depths and scores each
//! sample with `PctPlanetDesirability()`. PR8 owns the authoritative planet
//! economy; PR5 takes the desirability function as an injected callback so the
//! builder remains testable and does not silently claim PR8's rule is final.
//!
//! [`ReferenceHabitability`] uses the integer reconstruction of
//! `PctPlanetDesirability()` in `stars-decompile/planet.c`. It is included to
//! reproduce the documented Humanoids anchor (25 displayed points) and to give
//! the builder a stable live total. Its rounding is a documented reconstruction,
//! not an observed original result.

use crate::race::spec::RaceSpec;
use crate::race::traits::LesserTrait;

/// Supplies the raw innate-habitability value used by point step 2.
///
/// The value is the integer returned by `LInnateRaceHabitability()`, before the
/// `/2000` and growth-spread scaling in `CAdvantagePoints()`.
pub trait HabitabilityModel {
    /// Returns the raw innate-habitability estimate for a race.
    fn innate_habitability(&self, spec: &RaceSpec) -> i32;
}

/// The reference sampling model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReferenceHabitability;

impl HabitabilityModel for ReferenceHabitability {
    fn innate_habitability(&self, spec: &RaceSpec) -> i32 {
        innate_habitability_with(spec, reference_desirability)
    }
}

/// A model that returns a fixed innate value.
///
/// This exists for focused formula tests and for isolating terms before the
/// planet economy lands. It must not be presented as a recovered rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixedHabitability(pub i32);

impl HabitabilityModel for FixedHabitability {
    fn innate_habitability(&self, _spec: &RaceSpec) -> i32 {
        self.0
    }
}

/// Runs the three-pass sampling with an injected planet-desirability callback.
///
/// The callback receives the sample's gravity, temperature and radiation values
/// and the race being priced. It returns `PctPlanetDesirability()`, which may be
/// negative for an unusable world.
#[must_use]
pub fn innate_habitability_with<F>(spec: &RaceSpec, desirability: F) -> i32
where
    F: Fn([i32; 3], &RaceSpec) -> i32,
{
    let has_total_terraforming = spec.has_lesser(LesserTrait::TotalTerraforming);
    let env = [
        i32::from(spec.habitability[0].ideal()),
        i32::from(spec.habitability[1].ideal()),
        i32::from(spec.habitability[2].ideal()),
    ];

    // Numerator and common denominator for the final `lInnate` value. The
    // recovered routine scales with `/100.0` after each of two axes while
    // retaining fractions; fixed-point avoids platform-dependent float sums.
    let mut total_scaled: i128 = 0;

    // The decompilation keeps these deltas across passes and samples. They are
    // only updated for a non-immune axis once the terraforming depth is above
    // zero.
    let mut delta = [0_i32; 3];

    for pass in 0..3_i32 {
        let percent_terraforming = match pass {
            0 => 0,
            1 => {
                if has_total_terraforming {
                    8
                } else {
                    5
                }
            }
            _ => {
                if has_total_terraforming {
                    17
                } else {
                    15
                }
            }
        };

        let mut base = [0_i32; 3];
        let mut increment = [0_i32; 3];
        let mut steps = [1_i32; 3];

        for axis in 0..3 {
            if env[axis] < 0 {
                base[axis] = 50;
                increment[axis] = 11;
                steps[axis] = 1;
            } else {
                base[axis] = (i32::from(spec.habitability[axis].min) - percent_terraforming).max(0);
                let top = (i32::from(spec.habitability[axis].max) + percent_terraforming).min(100);
                increment[axis] = top - base[axis];
                steps[axis] = 11;
            }
        }

        // Denominator contributed by the two outer axis scalings.
        let mut denominator: i128 = 1;
        let mut terraforming_numerator: i128 = 0;

        for gravity_index in 0..steps[0] {
            let gravity = sample_value(base[0], increment[0], steps[0], gravity_index);
            let gravity =
                adjust_away_from_ideal(env[0], gravity, percent_terraforming, pass, &mut delta[0]);

            let mut axis_one_numerator: i128 = 0;

            for temperature_index in 0..steps[1] {
                let temperature = sample_value(base[1], increment[1], steps[1], temperature_index);
                let temperature = adjust_away_from_ideal(
                    env[1],
                    temperature,
                    percent_terraforming,
                    pass,
                    &mut delta[1],
                );

                let mut weighted_sum: i128 = 0;

                for radiation_index in 0..steps[2] {
                    let radiation = sample_value(base[2], increment[2], steps[2], radiation_index);
                    let radiation = adjust_away_from_ideal(
                        env[2],
                        radiation,
                        percent_terraforming,
                        pass,
                        &mut delta[2],
                    );

                    let mut desire = desirability([gravity, temperature, radiation], spec);
                    let delta_sum = delta[0] + delta[1] + delta[2];
                    if delta_sum > percent_terraforming {
                        desire -= delta_sum - percent_terraforming;
                        if desire < 0 {
                            desire = 0;
                        }
                    }

                    let weight = match pass {
                        0 => 7,
                        1 => 5,
                        _ => 6,
                    };
                    weighted_sum += i128::from(desire) * i128::from(desire) * i128::from(weight);
                }

                let axis_two_scaled = if env[2] >= 0 {
                    weighted_sum * i128::from(increment[2]) / 100
                } else {
                    weighted_sum * 11
                };
                axis_one_numerator += axis_two_scaled;
            }

            if env[1] >= 0 {
                axis_one_numerator *= i128::from(increment[1]);
                denominator *= 100;
            } else {
                axis_one_numerator *= 11;
            }
            terraforming_numerator += axis_one_numerator;
        }

        if env[0] >= 0 {
            terraforming_numerator *= i128::from(increment[0]);
            denominator *= 100;
        } else {
            terraforming_numerator *= 11;
        }

        // Normalize each pass to a denominator of 10,000 before summing.
        total_scaled += match denominator {
            1 => terraforming_numerator * 10_000,
            100 => terraforming_numerator * 100,
            _ => terraforming_numerator,
        };
    }

    // The recovered return is floor(lInnate / 10.0 + 0.5), where `lInnate`
    // itself is total_scaled / 10,000. The inputs are non-negative here.
    i32::try_from((total_scaled + 50_000) / 100_000).unwrap_or(i32::MAX)
}

fn sample_value(base: i32, increment: i32, steps: i32, index: i32) -> i32 {
    if index == 0 || steps <= 1 {
        base
    } else {
        base + (index * increment) / (steps - 1)
    }
}

fn adjust_away_from_ideal(
    ideal: i32,
    sample: i32,
    percent_terraforming: i32,
    pass: i32,
    delta: &mut i32,
) -> i32 {
    if pass == 0 || ideal < 0 {
        return sample;
    }

    let mut offset = ideal - sample;
    if offset.abs() <= percent_terraforming {
        offset = 0;
    } else if offset < 0 {
        offset += percent_terraforming;
    } else {
        offset -= percent_terraforming;
    }
    *delta = offset;
    ideal - offset
}

/// Integer reconstruction of `PctPlanetDesirability()`.
///
/// The environmental bands and ideal points come from the race; the planet
/// value is the sampled triple. A negative result is the recovered distance
/// penalty for a world outside the race's bands.
#[must_use]
pub fn reference_desirability(planet: [i32; 3], spec: &RaceSpec) -> i32 {
    let mut positive = 0_i64;
    let mut negative = 0_i32;
    let mut modifier = 10_000_i64;

    for (axis, &planet_value) in planet.iter().enumerate() {
        let ideal = i32::from(spec.habitability[axis].ideal());
        let min = i32::from(spec.habitability[axis].effective_min());
        let max = i32::from(spec.habitability[axis].effective_max());

        if max < 0 {
            positive += 10_000;
            continue;
        }

        if planet_value < min || planet_value > max {
            let distance = if planet_value < min {
                min - planet_value
            } else {
                planet_value - max
            };
            negative += distance.min(15);
            continue;
        }

        let absolute_difference = (planet_value - ideal).abs();
        let (range, penalty) = if planet_value < ideal {
            let range = ideal - min;
            (range, (ideal - planet_value) * 2 - range)
        } else {
            let range = max - ideal;
            (range, (planet_value - ideal) * 2 - range)
        };

        let percent_from_ideal = if range == 0 {
            100
        } else {
            absolute_difference * 100 / range
        };
        let percent_ideal = 100 - percent_from_ideal;
        positive += i64::from(percent_ideal) * i64::from(percent_ideal);

        if penalty > 0 && range > 0 {
            let denominator = range * 2;
            modifier = modifier * i64::from(denominator - penalty) / i64::from(denominator);
        }
    }

    if negative != 0 {
        return -negative;
    }

    // The recovered routine takes sqrt(pctPos / 3.0) and rounds its integer
    // conversion upward by 0.9 before applying the modifier. This is a
    // reconstruction of the decompiled code, not an observed value.
    let average = positive as f64 / 3.0;
    #[allow(clippy::cast_possible_truncation)]
    let base = (average.sqrt() + 0.9).trunc() as i64;
    #[allow(clippy::cast_possible_truncation)]
    let result = (base * modifier / 10_000) as i32;
    result
}

#[cfg(test)]
mod tests {
    use super::{
        FixedHabitability, HabitabilityModel, ReferenceHabitability, innate_habitability_with,
    };
    use crate::race::spec::{HabitabilityRange, RaceSpec};

    #[test]
    fn fixed_model_reports_the_supplied_value() {
        let spec = RaceSpec::humanoids_anchor();
        assert_eq!(FixedHabitability(1234).innate_habitability(&spec), 1234);
    }

    #[test]
    fn reference_model_reproduces_the_humanoids_anchor() {
        let spec = RaceSpec::humanoids_anchor();
        let innate = ReferenceHabitability.innate_habitability(&spec);
        assert_eq!(innate, 3_293_786);
    }

    #[test]
    fn total_terraforming_changes_the_sampling_depth() {
        let mut spec = RaceSpec::humanoids_anchor();
        let without = innate_habitability_with(&spec, super::reference_desirability);
        spec.lesser
            .insert(crate::race::traits::LesserTrait::TotalTerraforming);
        let with = innate_habitability_with(&spec, super::reference_desirability);
        assert_ne!(without, with);
    }

    #[test]
    fn immune_axis_contributes_a_fixed_positive_sample() {
        let mut spec = RaceSpec::humanoids_anchor();
        spec.habitability[0] = HabitabilityRange::immune_with_bounds(15, 85);
        let value = ReferenceHabitability.innate_habitability(&spec);
        assert!(value > 0);
    }
}
