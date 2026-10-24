//! Typed new-game settings for the recovered game-creation wizard.
//!
//! The value inventories and numeric ranges come from the January 1998 help
//! topics and the decompiled `GAME`/`rgvc` fields. The help describes the
//! original advanced wizard; this module keeps every recovered control rather
//! than silently dropping the ones whose effects land in later PRs.
//!
//! The defaults below are remake interface defaults, not recovered original
//! defaults. The original default victory set was not decoded.

use std::fmt;

/// Maximum number of computer opponents in a game.
pub const MAX_OPPONENTS: u8 = 15;
/// Maximum number of player slots, including the human.
pub const MAX_PLAYERS: u8 = 16;
/// Maximum game-name length in bytes, from the recovered `GAME.szName[32]`.
pub const GAME_NAME_MAX_BYTES: usize = 31;

/// Universe size, stored as original `mdSize` `0..=4`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GalaxySize {
    /// Tiny, 400 light years.
    Tiny,
    /// Small, 800 light years.
    Small,
    /// Medium, 1,200 light years.
    Medium,
    /// Large, 1,600 light years.
    Large,
    /// Huge, 2,000 light years.
    Huge,
}

impl GalaxySize {
    /// All sizes in stored order.
    pub const ALL: [Self; 5] = [
        Self::Tiny,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::Huge,
    ];

    /// Stored `mdSize` value.
    #[must_use]
    pub const fn stored_value(self) -> u8 {
        match self {
            Self::Tiny => 0,
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 3,
            Self::Huge => 4,
        }
    }

    /// Converts a stored value back to a size.
    #[must_use]
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Tiny),
            1 => Some(Self::Small),
            2 => Some(Self::Medium),
            3 => Some(Self::Large),
            4 => Some(Self::Huge),
            _ => None,
        }
    }

    /// Documented universe span in light years.
    #[must_use]
    pub const fn span_ly(self) -> i32 {
        self.stored_value() as i32 * 400 + 400
    }

    /// UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tiny => "Tiny (400 ly)",
            Self::Small => "Small (800 ly)",
            Self::Medium => "Medium (1,200 ly)",
            Self::Large => "Large (1,600 ly)",
            Self::Huge => "Huge (2,000 ly)",
        }
    }
}

/// Planet density, stored as original `mdDensity` `0..=3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Density {
    /// Sparse.
    Sparse,
    /// Normal.
    Normal,
    /// Dense.
    Dense,
    /// Packed.
    Packed,
}

impl Density {
    /// All densities in stored order.
    pub const ALL: [Self; 4] = [Self::Sparse, Self::Normal, Self::Dense, Self::Packed];

    /// Stored `mdDensity` value.
    #[must_use]
    pub const fn stored_value(self) -> u8 {
        match self {
            Self::Sparse => 0,
            Self::Normal => 1,
            Self::Dense => 2,
            Self::Packed => 3,
        }
    }

    /// Converts a stored value back to a density.
    #[must_use]
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Sparse),
            1 => Some(Self::Normal),
            2 => Some(Self::Dense),
            3 => Some(Self::Packed),
            _ => None,
        }
    }

    /// UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sparse => "Sparse",
            Self::Normal => "Normal",
            Self::Dense => "Dense",
            Self::Packed => "Packed",
        }
    }
}

/// Starting player distance, stored as original `mdStartDist` `0..=3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlayerPositions {
    /// Close starting positions.
    Close,
    /// Moderate starting positions.
    Moderate,
    /// Farther starting positions.
    Farther,
    /// Distant starting positions.
    Distant,
}

impl PlayerPositions {
    /// All choices in stored order.
    pub const ALL: [Self; 4] = [Self::Close, Self::Moderate, Self::Farther, Self::Distant];

    /// Stored `mdStartDist` value.
    #[must_use]
    pub const fn stored_value(self) -> u8 {
        match self {
            Self::Close => 0,
            Self::Moderate => 1,
            Self::Farther => 2,
            Self::Distant => 3,
        }
    }

    /// Converts a stored value back to a choice.
    #[must_use]
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Close),
            1 => Some(Self::Moderate),
            2 => Some(Self::Farther),
            3 => Some(Self::Distant),
            _ => None,
        }
    }

    /// UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Close => "Close",
            Self::Moderate => "Moderate",
            Self::Farther => "Farther",
            Self::Distant => "Distant",
        }
    }
}

/// Computer-opponent proficiency, stored as original AI level `0..=3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Difficulty {
    /// Easy.
    Easy,
    /// Standard.
    Standard,
    /// Tough.
    Tough,
    /// Expert.
    Expert,
}

impl Difficulty {
    /// All levels in stored order.
    pub const ALL: [Self; 4] = [Self::Easy, Self::Standard, Self::Tough, Self::Expert];

    /// Stored AI level.
    #[must_use]
    pub const fn stored_value(self) -> u8 {
        match self {
            Self::Easy => 0,
            Self::Standard => 1,
            Self::Tough => 2,
            Self::Expert => 3,
        }
    }

    /// Converts a stored value back to a level.
    #[must_use]
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Easy),
            1 => Some(Self::Standard),
            2 => Some(Self::Tough),
            3 => Some(Self::Expert),
            _ => None,
        }
    }

    /// UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Standard => "Standard",
            Self::Tough => "Tough",
            Self::Expert => "Expert",
        }
    }
}

/// Advanced game-creation option bits.
///
/// The field names follow the recovered `GAME` bit-field names. The labels are
/// the help's player-facing names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameOptions {
    /// `fExtraFuel`: the help calls this Beginner: Maximum Minerals. Every
    /// planet starts with mineral concentration 100.
    pub maximum_minerals: bool,
    /// `fSlowTech`: research costs twice as much.
    pub slower_tech: bool,
    /// `fBBSPlay`: accelerated BBS play.
    pub accelerated_bbs: bool,
    /// `fNoRandom`: disables random events and wormholes.
    pub no_random_events: bool,
    /// `fAisBand`: computer players prefer attacking the human.
    pub computer_alliances: bool,
    /// `fVisScores`: public player scores after turn 20.
    pub public_scores: bool,
}

impl GameOptions {
    /// Returns the six options as labelled pairs.
    #[must_use]
    pub const fn labelled(self) -> [(&'static str, bool); 6] {
        [
            ("Beginner: Maximum Minerals", self.maximum_minerals),
            ("Slower Tech Advances", self.slower_tech),
            ("Accelerated BBS Play", self.accelerated_bbs),
            ("No Random Events", self.no_random_events),
            ("Computer Players Form Alliances", self.computer_alliances),
            ("Public Player Scores", self.public_scores),
        ]
    }
}

/// One of the seven recovered victory-condition kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VictoryConditionKind {
    /// Own a percentage of all planets.
    OwnsPercentPlanets,
    /// Attain a technology level in a number of fields.
    AttainsTechnology,
    /// Exceed a score.
    Score,
    /// Exceed the nearest rival's score by a percentage.
    ExceedsNearest,
    /// Reach a production capacity.
    ProductionCapacity,
    /// Own a number of capital ships.
    CapitalShips,
    /// Highest score after a number of years.
    Turns,
}

impl VictoryConditionKind {
    /// All kinds in the recovered dialog order.
    pub const ALL: [Self; 7] = [
        Self::OwnsPercentPlanets,
        Self::AttainsTechnology,
        Self::Score,
        Self::ExceedsNearest,
        Self::ProductionCapacity,
        Self::CapitalShips,
        Self::Turns,
    ];

    /// UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OwnsPercentPlanets => "Own a percentage of planets",
            Self::AttainsTechnology => "Attain technology level in fields",
            Self::Score => "Exceed a score",
            Self::ExceedsNearest => "Exceed the nearest rival by percent",
            Self::ProductionCapacity => "Reach production capacity",
            Self::CapitalShips => "Own capital ships",
            Self::Turns => "Highest score after years",
        }
    }
}

/// One victory condition with its enabled state and threshold.
///
/// Thresholds use the documented player-facing ranges. The stored raw values
/// are recovered through `GetVCVal()`; this type keeps the displayed values so
/// the settings remain readable and validate before generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VictoryCondition {
    /// Own a percentage of all planets, 20..=100.
    OwnsPercentPlanets {
        /// Whether the condition is enabled.
        enabled: bool,
        /// Required percentage.
        percent: u8,
    },
    /// Attain a technology level in a number of fields.
    AttainsTechnology {
        /// Whether the condition is enabled.
        enabled: bool,
        /// Required level, 8..=26.
        level: u8,
        /// Required field count, 2..=6.
        fields: u8,
    },
    /// Exceed an absolute score, 1000..=20000.
    Score {
        /// Whether the condition is enabled.
        enabled: bool,
        /// Required score.
        score: u16,
    },
    /// Exceed the nearest rival's score by a percentage, 20..=300.
    ExceedsNearest {
        /// Whether the condition is enabled.
        enabled: bool,
        /// Required percentage.
        percent: u16,
    },
    /// Reach production capacity, 10..=500.
    ProductionCapacity {
        /// Whether the condition is enabled.
        enabled: bool,
        /// Required capacity.
        capacity: u16,
    },
    /// Own capital ships, 10..=300.
    CapitalShips {
        /// Whether the condition is enabled.
        enabled: bool,
        /// Required ship count.
        count: u16,
    },
    /// Highest score after years, 30..=900.
    Turns {
        /// Whether the condition is enabled.
        enabled: bool,
        /// Required years.
        years: u16,
    },
}

impl VictoryCondition {
    /// Creates the default threshold for a kind.
    ///
    /// These values are remake interface defaults; the original defaults were
    /// not decoded.
    #[must_use]
    pub const fn default_for(kind: VictoryConditionKind) -> Self {
        match kind {
            VictoryConditionKind::OwnsPercentPlanets => Self::OwnsPercentPlanets {
                enabled: false,
                percent: 60,
            },
            VictoryConditionKind::AttainsTechnology => Self::AttainsTechnology {
                enabled: false,
                level: 10,
                fields: 3,
            },
            VictoryConditionKind::Score => Self::Score {
                enabled: false,
                score: 5_000,
            },
            VictoryConditionKind::ExceedsNearest => Self::ExceedsNearest {
                enabled: false,
                percent: 50,
            },
            VictoryConditionKind::ProductionCapacity => Self::ProductionCapacity {
                enabled: false,
                capacity: 100,
            },
            VictoryConditionKind::CapitalShips => Self::CapitalShips {
                enabled: false,
                count: 50,
            },
            VictoryConditionKind::Turns => Self::Turns {
                enabled: false,
                years: 100,
            },
        }
    }

    /// Kind of this condition.
    #[must_use]
    pub const fn kind(self) -> VictoryConditionKind {
        match self {
            Self::OwnsPercentPlanets { .. } => VictoryConditionKind::OwnsPercentPlanets,
            Self::AttainsTechnology { .. } => VictoryConditionKind::AttainsTechnology,
            Self::Score { .. } => VictoryConditionKind::Score,
            Self::ExceedsNearest { .. } => VictoryConditionKind::ExceedsNearest,
            Self::ProductionCapacity { .. } => VictoryConditionKind::ProductionCapacity,
            Self::CapitalShips { .. } => VictoryConditionKind::CapitalShips,
            Self::Turns { .. } => VictoryConditionKind::Turns,
        }
    }

    /// Whether the condition is enabled.
    #[must_use]
    pub const fn enabled(self) -> bool {
        match self {
            Self::OwnsPercentPlanets { enabled, .. }
            | Self::AttainsTechnology { enabled, .. }
            | Self::Score { enabled, .. }
            | Self::ExceedsNearest { enabled, .. }
            | Self::ProductionCapacity { enabled, .. }
            | Self::CapitalShips { enabled, .. }
            | Self::Turns { enabled, .. } => enabled,
        }
    }

    /// Sets the enabled flag.
    pub fn set_enabled(&mut self, value: bool) {
        match self {
            Self::OwnsPercentPlanets { enabled, .. }
            | Self::AttainsTechnology { enabled, .. }
            | Self::Score { enabled, .. }
            | Self::ExceedsNearest { enabled, .. }
            | Self::ProductionCapacity { enabled, .. }
            | Self::CapitalShips { enabled, .. }
            | Self::Turns { enabled, .. } => *enabled = value,
        }
    }

    /// Primary threshold as a number, for compact UI display.
    #[must_use]
    pub const fn primary_threshold(self) -> u16 {
        match self {
            Self::OwnsPercentPlanets { percent, .. } => percent as u16,
            Self::AttainsTechnology { level, .. } => level as u16,
            Self::Score { score, .. } => score,
            Self::ExceedsNearest { percent, .. } => percent,
            Self::ProductionCapacity { capacity, .. } => capacity,
            Self::CapitalShips { count, .. } => count,
            Self::Turns { years, .. } => years,
        }
    }

    /// Sets the primary threshold, clamped to its documented range and rounded
    /// to the nearest slider increment.
    pub fn set_primary_threshold(&mut self, value: u16) {
        let kind = self.kind();
        let (minimum, maximum) = Self::primary_range(kind);
        let step = Self::primary_step(kind);
        let clamped = value.clamp(minimum, maximum);
        let quantized = (minimum + (clamped - minimum + step / 2) / step * step).min(maximum);

        match self {
            Self::OwnsPercentPlanets { percent, .. } => {
                *percent = u8::try_from(quantized).unwrap_or(100);
            }
            Self::AttainsTechnology { level, .. } => {
                *level = u8::try_from(quantized).unwrap_or(26);
            }
            Self::Score { score, .. } => *score = quantized,
            Self::ExceedsNearest { percent, .. } => *percent = quantized,
            Self::ProductionCapacity { capacity, .. } => *capacity = quantized,
            Self::CapitalShips { count, .. } => *count = quantized,
            Self::Turns { years, .. } => *years = quantized,
        }
    }

    /// Documented primary range for a kind.
    #[must_use]
    pub const fn primary_range(kind: VictoryConditionKind) -> (u16, u16) {
        match kind {
            VictoryConditionKind::OwnsPercentPlanets => (20, 100),
            VictoryConditionKind::AttainsTechnology => (8, 26),
            VictoryConditionKind::Score => (1_000, 20_000),
            VictoryConditionKind::ExceedsNearest => (20, 300),
            VictoryConditionKind::ProductionCapacity => (10, 500),
            VictoryConditionKind::CapitalShips => (10, 300),
            VictoryConditionKind::Turns => (30, 900),
        }
    }

    /// Slider increment for a kind.
    ///
    /// Planet percentage and the two score-comparison thresholds use the
    /// requested five-point increments. Score uses the recovered 1,000-point
    /// step. Production capacity is displayed in thousands and advances in
    /// 10-thousand steps, matching the recovered `GetVCVal()` conversion.
    /// Capital ships use five-point increments as requested even though the
    /// recovered spinner advanced in tens.
    #[must_use]
    pub const fn primary_step(kind: VictoryConditionKind) -> u16 {
        match kind {
            VictoryConditionKind::OwnsPercentPlanets => 5,
            VictoryConditionKind::AttainsTechnology => 1,
            VictoryConditionKind::Score => 1_000,
            VictoryConditionKind::ExceedsNearest => 5,
            VictoryConditionKind::ProductionCapacity => 10,
            VictoryConditionKind::CapitalShips => 5,
            VictoryConditionKind::Turns => 10,
        }
    }

    fn validate(self, errors: &mut Vec<UniverseSettingsError>) {
        if !self.enabled() {
            return;
        }
        let kind = self.kind();
        let (min, max) = Self::primary_range(kind);
        let value = self.primary_threshold();
        if !(min..=max).contains(&value) {
            errors.push(UniverseSettingsError::VictoryValueOutOfRange {
                kind,
                value,
                min,
                max,
            });
        }
        if let Self::AttainsTechnology { fields, .. } = self
            && !(2..=6).contains(&fields)
        {
            errors.push(UniverseSettingsError::VictoryFieldsOutOfRange { value: fields });
        }
    }
}

impl Default for VictoryCondition {
    fn default() -> Self {
        Self::default_for(VictoryConditionKind::OwnsPercentPlanets)
    }
}

/// Step 3 victory settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VictorySettings {
    /// One condition per kind, in [`VictoryConditionKind::ALL`] order.
    pub conditions: [VictoryCondition; 7],
    /// Number of enabled conditions that must be met, 0..=7.
    pub must_meet: u8,
    /// Minimum game years before a winner is declared, 30..=500.
    pub minimum_years: u16,
}

impl VictorySettings {
    /// Remake defaults: no conditions enabled and the minimum-year floor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            conditions: [
                VictoryCondition::default_for(VictoryConditionKind::OwnsPercentPlanets),
                VictoryCondition::default_for(VictoryConditionKind::AttainsTechnology),
                VictoryCondition::default_for(VictoryConditionKind::Score),
                VictoryCondition::default_for(VictoryConditionKind::ExceedsNearest),
                VictoryCondition::default_for(VictoryConditionKind::ProductionCapacity),
                VictoryCondition::default_for(VictoryConditionKind::CapitalShips),
                VictoryCondition::default_for(VictoryConditionKind::Turns),
            ],
            must_meet: 0,
            minimum_years: 30,
        }
    }

    /// Returns the condition for one kind.
    #[must_use]
    pub fn condition(&self, kind: VictoryConditionKind) -> &VictoryCondition {
        let index = VictoryConditionKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap_or(0);
        &self.conditions[index]
    }

    /// Returns a mutable condition for one kind.
    pub fn condition_mut(&mut self, kind: VictoryConditionKind) -> &mut VictoryCondition {
        let index = VictoryConditionKind::ALL
            .iter()
            .position(|candidate| *candidate == kind)
            .unwrap_or(0);
        &mut self.conditions[index]
    }

    /// Number of enabled conditions.
    #[must_use]
    pub fn enabled_count(&self) -> u8 {
        self.conditions
            .iter()
            .filter(|condition| condition.enabled())
            .count()
            .try_into()
            .unwrap_or(u8::MAX)
    }

    fn validate(&self, errors: &mut Vec<UniverseSettingsError>) {
        for condition in self.conditions {
            condition.validate(errors);
        }
        if self.must_meet > 7 {
            errors.push(UniverseSettingsError::MustMeetOutOfRange {
                value: self.must_meet,
            });
        }
        if self.must_meet > self.enabled_count() {
            errors.push(UniverseSettingsError::MustMeetExceedsEnabled {
                must_meet: self.must_meet,
                enabled: self.enabled_count(),
            });
        }
        if !(30..=500).contains(&self.minimum_years) {
            errors.push(UniverseSettingsError::MinimumYearsOutOfRange {
                value: self.minimum_years,
            });
        }
    }
}

impl Default for VictorySettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete typed settings for one new game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniverseSettings {
    /// Game name shown in the title bar.
    pub game_name: String,
    /// Explicit universe-generation seed.
    pub seed: u64,
    /// Galaxy size.
    pub size: GalaxySize,
    /// Planet density.
    pub density: Density,
    /// Starting player distance.
    pub player_positions: PlayerPositions,
    /// Whether star systems are clustered.
    pub clumping: bool,
    /// Number of Turindrones opponents, 1..=15.
    pub opponent_count: u8,
    /// Computer-opponent proficiency.
    pub difficulty: Difficulty,
    /// Advanced option bits.
    pub options: GameOptions,
    /// Step 3 victory conditions.
    pub victory: VictorySettings,
}

impl UniverseSettings {
    /// Remake interface defaults for a new form.
    #[must_use]
    pub fn new() -> Self {
        Self {
            game_name: "New Game".to_owned(),
            seed: 1,
            size: GalaxySize::Medium,
            density: Density::Normal,
            player_positions: PlayerPositions::Moderate,
            clumping: false,
            opponent_count: 1,
            difficulty: Difficulty::Standard,
            options: GameOptions::default(),
            victory: VictorySettings::new(),
        }
    }

    /// Total player count including the human.
    #[must_use]
    pub const fn player_count(&self) -> u8 {
        self.opponent_count.saturating_add(1)
    }

    /// Number of planets generated before homeworld assignment.
    ///
    /// This follows the recovered `GenerateWorld()` count:
    /// `dGal * dGal / 5000`, then the density adjustment, capped at 999.
    #[must_use]
    pub const fn planet_count(&self) -> u16 {
        let span = self.size.span_ly();
        let base = (span * span / 5_000) as u16;
        let count = match self.density {
            Density::Sparse => base - base / 4,
            Density::Normal => base,
            Density::Dense => base + base / 4,
            Density::Packed => base + base / 4 + base / 4,
        };
        if count > 999 { 999 } else { count }
    }

    /// Validates all recovered ranges and combinations before generation.
    ///
    /// # Errors
    ///
    /// Returns every detected error in a stable order.
    pub fn validate(&self) -> Result<(), Vec<UniverseSettingsError>> {
        let mut errors = Vec::new();
        if self.game_name.trim().is_empty() {
            errors.push(UniverseSettingsError::BlankGameName);
        }
        if self.game_name.len() > GAME_NAME_MAX_BYTES {
            errors.push(UniverseSettingsError::GameNameTooLong {
                bytes: self.game_name.len(),
            });
        }
        if !(1..=MAX_OPPONENTS).contains(&self.opponent_count) {
            errors.push(UniverseSettingsError::OpponentCountOutOfRange {
                count: self.opponent_count,
            });
        }
        if self.player_count() > MAX_PLAYERS {
            errors.push(UniverseSettingsError::PlayerCountOutOfRange {
                count: self.player_count(),
            });
        }
        self.victory.validate(&mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for UniverseSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// A new-game setting that cannot be used for generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniverseSettingsError {
    /// The game name is empty or whitespace.
    BlankGameName,
    /// The game name exceeds the recovered 32-byte field.
    GameNameTooLong {
        /// Measured byte length.
        bytes: usize,
    },
    /// The opponent count is outside 1..=15.
    OpponentCountOutOfRange {
        /// Supplied count.
        count: u8,
    },
    /// The total player count exceeds 16.
    PlayerCountOutOfRange {
        /// Supplied total.
        count: u8,
    },
    /// A victory threshold is outside its documented range.
    VictoryValueOutOfRange {
        /// Condition kind.
        kind: VictoryConditionKind,
        /// Supplied value.
        value: u16,
        /// Minimum accepted value.
        min: u16,
        /// Maximum accepted value.
        max: u16,
    },
    /// The technology-field count is outside 2..=6.
    VictoryFieldsOutOfRange {
        /// Supplied value.
        value: u8,
    },
    /// The must-meet count is outside 0..=7.
    MustMeetOutOfRange {
        /// Supplied value.
        value: u8,
    },
    /// More conditions must be met than are enabled.
    MustMeetExceedsEnabled {
        /// Supplied must-meet count.
        must_meet: u8,
        /// Enabled condition count.
        enabled: u8,
    },
    /// The minimum-year floor is outside 30..=500.
    MinimumYearsOutOfRange {
        /// Supplied value.
        value: u16,
    },
}

impl fmt::Display for UniverseSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankGameName => formatter.write_str("game name must not be empty"),
            Self::GameNameTooLong { bytes } => write!(
                formatter,
                "game name is {bytes} bytes; the limit is {GAME_NAME_MAX_BYTES}"
            ),
            Self::OpponentCountOutOfRange { count } => {
                write!(
                    formatter,
                    "opponent count {count} is outside 1..={MAX_OPPONENTS}"
                )
            }
            Self::PlayerCountOutOfRange { count } => {
                write!(formatter, "player count {count} exceeds {MAX_PLAYERS}")
            }
            Self::VictoryValueOutOfRange {
                kind,
                value,
                min,
                max,
            } => write!(
                formatter,
                "{} value {value} is outside {min}..={max}",
                kind.label()
            ),
            Self::VictoryFieldsOutOfRange { value } => {
                write!(formatter, "technology field count {value} is outside 2..=6")
            }
            Self::MustMeetOutOfRange { value } => {
                write!(formatter, "must-meet count {value} is outside 0..=7")
            }
            Self::MustMeetExceedsEnabled { must_meet, enabled } => write!(
                formatter,
                "must-meet count {must_meet} exceeds the {enabled} enabled conditions"
            ),
            Self::MinimumYearsOutOfRange { value } => {
                write!(formatter, "minimum years {value} is outside 30..=500")
            }
        }
    }
}

impl std::error::Error for UniverseSettingsError {}

#[cfg(test)]
mod tests {
    use super::{Density, GalaxySize, UniverseSettings};

    #[test]
    fn recovered_planet_counts_cover_every_size_and_density() {
        let mut settings = UniverseSettings::new();
        for (size, expected) in [
            (GalaxySize::Tiny, [24_u16, 32, 40, 48]),
            (GalaxySize::Small, [96, 128, 160, 192]),
            (GalaxySize::Medium, [216, 288, 360, 432]),
            (GalaxySize::Large, [384, 512, 640, 768]),
            (GalaxySize::Huge, [600, 800, 999, 999]),
        ] {
            settings.size = size;
            for (density, count) in Density::ALL.into_iter().zip(expected) {
                settings.density = density;
                assert_eq!(settings.planet_count(), count, "{size:?} {density:?}");
            }
        }
    }

    #[test]
    fn victory_slider_steps_match_the_requested_increments() {
        use super::{VictoryCondition, VictoryConditionKind};

        assert_eq!(
            VictoryCondition::primary_step(VictoryConditionKind::OwnsPercentPlanets),
            5
        );
        assert_eq!(
            VictoryCondition::primary_step(VictoryConditionKind::ExceedsNearest),
            5
        );
        assert_eq!(
            VictoryCondition::primary_step(VictoryConditionKind::CapitalShips),
            5
        );
        assert_eq!(
            VictoryCondition::primary_step(VictoryConditionKind::Score),
            1_000
        );
        assert_eq!(
            VictoryCondition::primary_step(VictoryConditionKind::ProductionCapacity),
            10
        );
        assert_eq!(
            VictoryCondition::primary_step(VictoryConditionKind::Turns),
            10
        );

        let mut planets = VictoryCondition::OwnsPercentPlanets {
            enabled: true,
            percent: 20,
        };
        planets.set_primary_threshold(23);
        assert_eq!(planets.primary_threshold(), 25);
        let mut ships = VictoryCondition::CapitalShips {
            enabled: true,
            count: 10,
        };
        ships.set_primary_threshold(12);
        assert_eq!(ships.primary_threshold(), 10);
        let mut score = VictoryCondition::Score {
            enabled: true,
            score: 1_000,
        };
        score.set_primary_threshold(5_600);
        assert_eq!(score.primary_threshold(), 6_000);
    }

    #[test]
    fn invalid_ranges_and_victory_combinations_are_rejected() {
        let mut settings = UniverseSettings::new();
        settings.game_name.clear();
        settings.opponent_count = 16;
        settings.victory.must_meet = 2;
        let errors = settings.validate().expect_err("invalid settings");
        assert!(errors.len() >= 3);

        settings = UniverseSettings::new();
        settings.victory.conditions[0] = super::VictoryCondition::OwnsPercentPlanets {
            enabled: true,
            percent: 10,
        };
        assert!(settings.validate().is_err());
    }
}
