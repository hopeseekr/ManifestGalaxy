//! Versioned JSON persistence for a validated custom race.
//!
//! This is a focused race-file format, not the future game-save envelope. It
//! lives in the desktop package because core has no filesystem dependency.
//! Files are decoded into the typed core model and passed through
//! [`validate_race`] before they can be used.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use stars_core::race::{
    Efficiency, HabitabilityRange, LeftoverAllocation, LesserTrait, PrimaryTrait, RaceSpec,
    RaceValidationError, ReferenceHabitability, ResearchCost, ResearchSettings, validate_race,
};

/// Marker written into every race JSON file.
pub const RACE_FILE_FORMAT: &str = "stars-remake-race";
/// Supported race-file envelope version.
pub const RACE_FILE_VERSION: u32 = 1;

/// Failure to save, load or validate a race JSON file.
#[derive(Debug)]
pub enum RaceFileError {
    /// The file could not be read or written.
    Io(io::Error),
    /// The JSON was malformed.
    Json(serde_json::Error),
    /// The file is not a Stars! remake race file.
    UnsupportedFormat(String),
    /// The file uses a future or unknown envelope version.
    UnsupportedVersion(u32),
    /// The race fields or advantage-point balance are invalid.
    InvalidRace(Vec<RaceValidationError>),
    /// A stored enum value has no defined meaning.
    InvalidValue {
        /// Field name.
        field: &'static str,
        /// Offending value.
        value: u64,
    },
}

impl fmt::Display for RaceFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "race file I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "race file JSON is invalid: {error}"),
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported race file format {format:?}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported race file version {version}")
            }
            Self::InvalidRace(errors) => {
                formatter.write_str("race file contains an invalid race")?;
                for error in errors {
                    write!(formatter, ": {error}")?;
                }
                Ok(())
            }
            Self::InvalidValue { field, value } => {
                write!(formatter, "invalid {field} value {value}")
            }
        }
    }
}

impl std::error::Error for RaceFileError {}

impl From<io::Error> for RaceFileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for RaceFileError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RaceDocument {
    format: String,
    version: u32,
    name: String,
    plural_name: String,
    emblem: u8,
    password: String,
    primary: u8,
    lesser: Vec<u8>,
    growth_rate: u8,
    habitability: [HabitabilityAxisDocument; 3],
    efficiency: EfficiencyDocument,
    cheap_factories: bool,
    research_costs: [u8; 6],
    tech3: bool,
    leftover: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HabitabilityAxisDocument {
    min: u8,
    max: u8,
    immune: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EfficiencyDocument {
    resource_generation: u8,
    factory_production: u8,
    factory_cost: u8,
    factory_operation: u8,
    mine_production: u8,
    mine_cost: u8,
    mine_operation: u8,
}

impl From<&RaceSpec> for RaceDocument {
    fn from(spec: &RaceSpec) -> Self {
        let habitability = spec.habitability.map(|band| HabitabilityAxisDocument {
            min: band.min,
            max: band.max,
            immune: band.immune,
        });
        Self {
            format: RACE_FILE_FORMAT.to_owned(),
            version: RACE_FILE_VERSION,
            name: spec.name.clone(),
            plural_name: spec.plural_name.clone(),
            emblem: spec.emblem,
            password: spec.password.clone(),
            primary: spec.primary.attribute_index(),
            lesser: spec.lesser.iter().map(|value| value.bit_index()).collect(),
            growth_rate: spec.growth_rate,
            habitability,
            efficiency: EfficiencyDocument {
                resource_generation: spec.efficiency.resource_generation,
                factory_production: spec.efficiency.factory_production,
                factory_cost: spec.efficiency.factory_cost,
                factory_operation: spec.efficiency.factory_operation,
                mine_production: spec.efficiency.mine_production,
                mine_cost: spec.efficiency.mine_cost,
                mine_operation: spec.efficiency.mine_operation,
            },
            cheap_factories: spec.cheap_factories,
            research_costs: spec.research.costs.map(|cost| cost.value()),
            tech3: spec.research.tech3,
            leftover: spec.leftover.value(),
        }
    }
}

impl RaceDocument {
    fn into_spec(self) -> Result<RaceSpec, RaceFileError> {
        if self.format != RACE_FILE_FORMAT {
            return Err(RaceFileError::UnsupportedFormat(self.format));
        }
        if self.version != RACE_FILE_VERSION {
            return Err(RaceFileError::UnsupportedVersion(self.version));
        }

        let primary = PrimaryTrait::from_attribute_index(self.primary).ok_or(
            RaceFileError::InvalidValue {
                field: "primary trait",
                value: u64::from(self.primary),
            },
        )?;

        let mut lesser = BTreeSet::new();
        for bit in self.lesser {
            let trait_value =
                LesserTrait::from_bit_index(bit).ok_or(RaceFileError::InvalidValue {
                    field: "lesser trait",
                    value: u64::from(bit),
                })?;
            if !lesser.insert(trait_value) {
                return Err(RaceFileError::InvalidValue {
                    field: "duplicate lesser trait",
                    value: u64::from(bit),
                });
            }
        }

        let mut costs = [ResearchCost::Standard; 6];
        for (index, value) in self.research_costs.into_iter().enumerate() {
            costs[index] = ResearchCost::from_value(value).ok_or(RaceFileError::InvalidValue {
                field: "research cost",
                value: u64::from(value),
            })?;
        }

        let leftover =
            LeftoverAllocation::from_value(self.leftover).ok_or(RaceFileError::InvalidValue {
                field: "leftover allocation",
                value: u64::from(self.leftover),
            })?;

        let habitability = self.habitability.map(|band| HabitabilityRange {
            min: band.min,
            max: band.max,
            immune: band.immune,
        });

        let spec = RaceSpec {
            name: self.name,
            plural_name: self.plural_name,
            emblem: self.emblem,
            password: self.password,
            primary,
            lesser,
            growth_rate: self.growth_rate,
            habitability,
            efficiency: Efficiency {
                resource_generation: self.efficiency.resource_generation,
                factory_production: self.efficiency.factory_production,
                factory_cost: self.efficiency.factory_cost,
                factory_operation: self.efficiency.factory_operation,
                mine_production: self.efficiency.mine_production,
                mine_cost: self.efficiency.mine_cost,
                mine_operation: self.efficiency.mine_operation,
            },
            cheap_factories: self.cheap_factories,
            research: ResearchSettings {
                costs,
                tech3: self.tech3,
            },
            leftover,
        };

        validate_race(&spec, &ReferenceHabitability).map_err(RaceFileError::InvalidRace)?;
        Ok(spec)
    }
}

/// Serializes a validated race to pretty JSON.
///
/// # Errors
///
/// Returns [`RaceFileError::InvalidRace`] if the race fails builder
/// validation, or a JSON error if serialization fails.
pub fn encode_race_json(spec: &RaceSpec) -> Result<String, RaceFileError> {
    validate_race(spec, &ReferenceHabitability).map_err(RaceFileError::InvalidRace)?;
    Ok(serde_json::to_string_pretty(&RaceDocument::from(spec))?)
}

/// Parses and validates a race from JSON.
///
/// # Errors
///
/// Returns a typed error for malformed JSON, unsupported format or version,
/// invalid stored values, or a race that fails builder validation.
pub fn decode_race_json(text: &str) -> Result<RaceSpec, RaceFileError> {
    let document: RaceDocument = serde_json::from_str(text)?;
    document.into_spec()
}

/// Writes a validated race to `path` using a temporary file and rename.
///
/// # Errors
///
/// Returns an error when validation fails, the destination directory cannot be
/// created, or the temporary file cannot be written or renamed.
pub fn save_race_json(spec: &RaceSpec, path: &Path) -> Result<(), RaceFileError> {
    let json = encode_race_json(spec)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_path(path);
    fs::write(&temporary, json)?;
    if let Err(error) = fs::rename(&temporary, path) {
        if path.exists() {
            // Windows cannot replace an existing destination with rename.
            fs::remove_file(path)?;
            fs::rename(&temporary, path)?;
        } else {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
    }
    Ok(())
}

/// Reads and validates a race from `path`.
///
/// # Errors
///
/// Returns the same typed errors as [`decode_race_json`], plus file I/O errors.
pub fn load_race_json(path: &Path) -> Result<RaceSpec, RaceFileError> {
    let text = fs::read_to_string(path)?;
    decode_race_json(&text)
}

/// Returns a suggested JSON file name for a race name.
#[must_use]
pub fn suggested_race_file_name(race_name: &str) -> String {
    format!("{}.json", file_slug(race_name))
}

/// Returns the default save path for a race name.
#[must_use]
pub fn default_race_path(spec: &RaceSpec) -> PathBuf {
    PathBuf::from("races").join(suggested_race_file_name(&spec.name))
}

fn file_slug(name: &str) -> String {
    let mut slug = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "race".to_owned()
    } else {
        slug.to_owned()
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    PathBuf::from(temporary)
}

#[cfg(test)]
mod tests {
    use super::{decode_race_json, encode_race_json, file_slug};
    use stars_core::race::{LesserTrait, RaceSpec, ReferenceHabitability, validate_race};

    #[test]
    fn json_round_trip_preserves_the_race() {
        let mut spec = RaceSpec::humanoids_anchor();
        spec.name = "JSON Test".to_owned();
        spec.lesser.insert(LesserTrait::TotalTerraforming);
        spec.lesser.insert(LesserTrait::MineralAlchemy);
        spec.growth_rate = 1;
        spec.research.costs[0] = stars_core::race::ResearchCost::Extra;

        let json = encode_race_json(&spec).expect("encode");
        assert!(json.contains("\"format\": \"stars-remake-race\""));
        let decoded = decode_race_json(&json).expect("decode");
        assert_eq!(decoded, spec);
        assert!(validate_race(&decoded, &ReferenceHabitability).is_ok());
    }

    #[test]
    fn save_and_load_round_trip_uses_a_file() {
        let mut spec = RaceSpec::humanoids_anchor();
        spec.name = "Round Trip".to_owned();
        let path =
            std::env::temp_dir().join(format!("stars-race-round-trip-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        super::save_race_json(&spec, &path).expect("save");
        let loaded = super::load_race_json(&path).expect("load");
        std::fs::remove_file(&path).expect("remove fixture");

        assert_eq!(loaded, spec);
    }

    #[test]
    fn future_versions_and_unknown_formats_are_rejected() {
        let spec = RaceSpec::humanoids_anchor();
        let json = encode_race_json(&spec).expect("encode");
        let future = json.replace("\"version\": 1", "\"version\": 999");
        assert!(matches!(
            decode_race_json(&future),
            Err(super::RaceFileError::UnsupportedVersion(999))
        ));

        let wrong_format = json.replace("stars-remake-race", "another-game");
        assert!(matches!(
            decode_race_json(&wrong_format),
            Err(super::RaceFileError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn slug_is_safe_for_a_file_name() {
        assert_eq!(file_slug("My Race!"), "my-race");
        assert_eq!(file_slug("___"), "race");
        assert_eq!(super::suggested_race_file_name("My Race!"), "my-race.json");
    }
}
