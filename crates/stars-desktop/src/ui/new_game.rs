//! Desktop new-game form and universe-generation worker.
//!
//! The form edits [`UniverseSettings`] and displays the validated [`RaceSpec`]
//! supplied by the custom-race builder. All validation and generation happen in
//! `stars-core`. The worker owns the request/response discipline: the active
//! universe is replaced only by a completed result, so a validation or worker
//! failure leaves the previous game intact.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use eframe::egui;

use stars_core::race::{RaceSpec, RaceValidationError, ReferenceHabitability, validate_race};
use stars_core::universe::{
    GeneratedUniverse, UniverseGenerationError, UniverseGenerationStage, UniverseProgress,
    UniverseSettings, UniverseSettingsError, VictoryCondition, VictoryConditionKind,
    generate_universe_with_progress,
};

use crate::race_file::load_race_json;
use crate::worker::RequestId;

/// A typed failure from a universe-generation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniverseWorkerError {
    /// Core rejected the settings or race.
    Generation(UniverseGenerationError),
    /// The request identifier was not newer than the last accepted request.
    StaleRequest {
        /// Rejected request.
        request: RequestId,
        /// Last request seen by the worker.
        last_request: Option<RequestId>,
    },
    /// The command channel is closed.
    CommandChannelClosed,
    /// The operating system refused to start the worker thread.
    SpawnFailed {
        /// Operating-system detail.
        message: String,
    },
}

impl fmt::Display for UniverseWorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generation(error) => write!(formatter, "{error}"),
            Self::StaleRequest {
                request,
                last_request,
            } => match last_request {
                Some(last) => write!(
                    formatter,
                    "{request} is not newer than worker request {last}"
                ),
                None => write!(formatter, "{request} is not a fresh request"),
            },
            Self::CommandChannelClosed => {
                write!(formatter, "universe worker command channel is closed")
            }
            Self::SpawnFailed { message } => {
                write!(
                    formatter,
                    "failed to start universe worker thread: {message}"
                )
            }
        }
    }
}

impl std::error::Error for UniverseWorkerError {}

/// One universe-generation job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniverseJob {
    /// Request identifier echoed in every message.
    pub request_id: RequestId,
    /// Settings for the new game.
    pub settings: UniverseSettings,
    /// Validated human race.
    pub race: RaceSpec,
}

/// A message produced by the universe worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniverseWorkerMessage {
    /// Generation reached a checkpoint.
    Progress {
        /// Request that produced the progress.
        request_id: RequestId,
        /// Core progress checkpoint.
        progress: UniverseProgress,
    },
    /// Generation completed and produced a universe.
    Completed {
        /// Request that completed.
        request_id: RequestId,
        /// Complete starting universe.
        universe: Box<GeneratedUniverse>,
    },
    /// Generation failed without producing a universe.
    Failed {
        /// Request that failed.
        request_id: RequestId,
        /// Typed failure.
        error: UniverseWorkerError,
    },
}

impl UniverseWorkerMessage {
    /// Request identifier attached to the message.
    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        match self {
            Self::Progress { request_id, .. }
            | Self::Completed { request_id, .. }
            | Self::Failed { request_id, .. } => *request_id,
        }
    }
}

enum UniverseCommand {
    Submit(UniverseJob),
}

/// Handle to the desktop universe-generation worker.
pub struct UniverseWorkerHandle {
    commands: Option<Sender<UniverseCommand>>,
    join: Option<JoinHandle<()>>,
}

impl UniverseWorkerHandle {
    /// Starts the worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`UniverseWorkerError::SpawnFailed`] when the thread cannot be
    /// created.
    pub fn spawn() -> Result<(Self, Receiver<UniverseWorkerMessage>), UniverseWorkerError> {
        let (command_sender, command_receiver) = mpsc::channel();
        let (message_sender, message_receiver) = mpsc::channel();

        let join = thread::Builder::new()
            .name("stars-universe-worker".to_owned())
            .spawn(move || worker_loop(&command_receiver, &message_sender))
            .map_err(|error| UniverseWorkerError::SpawnFailed {
                message: error.to_string(),
            })?;

        Ok((
            Self {
                commands: Some(command_sender),
                join: Some(join),
            },
            message_receiver,
        ))
    }

    /// Submits one job.
    ///
    /// # Errors
    ///
    /// Returns [`UniverseWorkerError::CommandChannelClosed`] when the worker
    /// has stopped.
    pub fn submit(&self, job: UniverseJob) -> Result<(), UniverseWorkerError> {
        self.commands
            .as_ref()
            .ok_or(UniverseWorkerError::CommandChannelClosed)?
            .send(UniverseCommand::Submit(job))
            .map_err(|_| UniverseWorkerError::CommandChannelClosed)
    }

    /// Stops the worker and joins its thread.
    pub fn shutdown(mut self) {
        self.commands.take();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for UniverseWorkerHandle {
    fn drop(&mut self) {
        self.commands.take();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn worker_loop(commands: &Receiver<UniverseCommand>, messages: &Sender<UniverseWorkerMessage>) {
    let mut last_request: Option<RequestId> = None;
    while let Ok(UniverseCommand::Submit(job)) = commands.recv() {
        if last_request.is_some_and(|last| job.request_id <= last) {
            let _ = messages.send(UniverseWorkerMessage::Failed {
                request_id: job.request_id,
                error: UniverseWorkerError::StaleRequest {
                    request: job.request_id,
                    last_request,
                },
            });
            continue;
        }
        last_request = Some(job.request_id);

        let result = generate_universe_with_progress(&job.settings, &job.race, |progress| {
            let _ = messages.send(UniverseWorkerMessage::Progress {
                request_id: job.request_id,
                progress,
            });
        });

        match result {
            Ok(universe) => {
                if messages
                    .send(UniverseWorkerMessage::Completed {
                        request_id: job.request_id,
                        universe: Box::new(universe),
                    })
                    .is_err()
                {
                    return;
                }
            }
            Err(error) => {
                if messages
                    .send(UniverseWorkerMessage::Failed {
                        request_id: job.request_id,
                        error: UniverseWorkerError::Generation(error),
                    })
                    .is_err()
                {
                    return;
                }
            }
        }
    }
}

/// One form validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewGameError {
    /// No validated custom race is available.
    MissingRace,
    /// The race fields or advantage-point balance are invalid.
    InvalidRace(Vec<RaceValidationError>),
    /// The universe settings are invalid.
    InvalidSettings(Vec<UniverseSettingsError>),
    /// The seed text is not a non-negative 64-bit integer.
    SeedNotANumber(String),
    /// A race file could not be read, decoded or validated.
    RaceFile {
        /// Path that was attempted.
        path: String,
        /// Typed race-file error text.
        message: String,
    },
    /// The worker rejected the request.
    Worker(String),
}

impl fmt::Display for NewGameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRace => {
                formatter.write_str("build and finish a custom race before creating a new game")
            }
            Self::InvalidRace(errors) => {
                formatter.write_str("custom race is invalid")?;
                for error in errors {
                    write!(formatter, ": {error}")?;
                }
                Ok(())
            }
            Self::InvalidSettings(errors) => {
                formatter.write_str("universe settings are invalid")?;
                for error in errors {
                    write!(formatter, ": {error}")?;
                }
                Ok(())
            }
            Self::SeedNotANumber(text) => {
                write!(formatter, "seed {text:?} is not a 64-bit integer")
            }
            Self::RaceFile { path, message } => {
                write!(formatter, "race file {path:?} could not be used: {message}")
            }
            Self::Worker(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for NewGameError {}

/// Current new-game activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewGameActivity {
    /// No request has been made.
    Idle,
    /// The form failed validation; the previous universe is retained.
    Invalid,
    /// The worker is generating.
    Generating {
        /// Active request.
        request_id: RequestId,
        /// Latest checkpoint.
        progress: Option<UniverseProgress>,
    },
    /// A universe was committed.
    Generated {
        /// Request that committed.
        request_id: RequestId,
    },
    /// The request failed; the previous universe is retained.
    Failed {
        /// Failed request.
        request_id: RequestId,
        /// Human-readable failure.
        message: String,
    },
}

impl NewGameActivity {
    /// Whether the worker is still generating.
    #[must_use]
    pub const fn is_generating(&self) -> bool {
        matches!(self, Self::Generating { .. })
    }
}

/// Headless desktop state for the new-game form and worker results.
pub struct NewGameState {
    settings: UniverseSettings,
    seed_text: String,
    race: Option<RaceSpec>,
    race_from_file: bool,
    race_file_path: String,
    available_race_files: Vec<PathBuf>,
    race_files_loaded: bool,
    errors: Vec<NewGameError>,
    universe: Option<GeneratedUniverse>,
    activity: NewGameActivity,
    worker: Option<UniverseWorkerHandle>,
    messages: Option<Receiver<UniverseWorkerMessage>>,
    next_request: u64,
    active_request: Option<RequestId>,
    last_accepted_request: Option<RequestId>,
}

impl Default for NewGameState {
    fn default() -> Self {
        Self::new()
    }
}

impl NewGameState {
    /// Creates an idle form with remake defaults.
    #[must_use]
    pub fn new() -> Self {
        let settings = UniverseSettings::new();
        Self {
            seed_text: settings.seed.to_string(),
            settings,
            race: None,
            race_from_file: false,
            race_file_path: String::new(),
            available_race_files: Vec::new(),
            race_files_loaded: false,
            errors: Vec::new(),
            universe: None,
            activity: NewGameActivity::Idle,
            worker: None,
            messages: None,
            next_request: 1,
            active_request: None,
            last_accepted_request: None,
        }
    }

    /// Current settings.
    #[must_use]
    pub const fn settings(&self) -> &UniverseSettings {
        &self.settings
    }

    /// Mutable settings for headless tests and UI controls.
    pub fn settings_mut(&mut self) -> &mut UniverseSettings {
        &mut self.settings
    }

    /// Current seed text.
    #[must_use]
    pub fn seed_text(&self) -> &str {
        &self.seed_text
    }

    /// Replaces the seed entry and settings value.
    pub fn set_seed(&mut self, seed: u64) {
        self.settings.seed = seed;
        self.seed_text = seed.to_string();
    }

    /// Mutable seed text for the UI.
    pub fn seed_text_mut(&mut self) -> &mut String {
        &mut self.seed_text
    }

    /// Validated race currently assigned to the human slot.
    #[must_use]
    pub const fn race(&self) -> Option<&RaceSpec> {
        self.race.as_ref()
    }

    /// Assigns or clears the human race from the race builder.
    pub fn set_race(&mut self, race: Option<RaceSpec>) {
        self.race_from_file = false;
        if self.race != race {
            self.race = race;
            self.errors.clear();
            if matches!(self.activity, NewGameActivity::Invalid) {
                self.activity = NewGameActivity::Idle;
            }
        }
    }

    /// Assigns the builder race unless a race file is explicitly selected.
    pub fn set_builder_race(&mut self, race: Option<RaceSpec>) {
        if !self.race_from_file {
            self.set_race(race);
        }
    }

    /// Whether the current race came from a loaded race file.
    #[must_use]
    pub const fn race_from_file(&self) -> bool {
        self.race_from_file
    }

    /// Current race-file path text.
    #[must_use]
    pub fn race_file_path(&self) -> &str {
        &self.race_file_path
    }

    /// Mutable race-file path text.
    pub fn race_file_path_mut(&mut self) -> &mut String {
        &mut self.race_file_path
    }

    /// Race JSON files discovered in `races/`.
    #[must_use]
    pub fn available_race_files(&self) -> &[PathBuf] {
        &self.available_race_files
    }

    /// Whether the race-file directory has been scanned.
    #[must_use]
    pub const fn race_files_loaded(&self) -> bool {
        self.race_files_loaded
    }

    /// Scans the default `races/` directory for race JSON files.
    pub fn refresh_race_files(&mut self) {
        self.refresh_race_files_in(Path::new("races"));
    }

    /// Scans an explicit directory for race JSON files.
    pub fn refresh_race_files_in(&mut self, directory: &Path) {
        self.race_files_loaded = true;
        self.available_race_files.clear();
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                self.available_race_files.push(path);
            }
        }
        self.available_race_files.sort();
        if self.race_file_path.trim().is_empty()
            && let Some(first) = self.available_race_files.first()
        {
            self.race_file_path = first.display().to_string();
        }
    }

    /// Loads, decodes and validates a race file.
    ///
    /// A failed load leaves the current race and universe untouched.
    ///
    /// # Errors
    ///
    /// Returns a [`NewGameError::RaceFile`] with the path and decoder detail.
    pub fn load_race_file(&mut self, path: &Path) -> Result<(), NewGameError> {
        match load_race_json(path) {
            Ok(race) => {
                self.race = Some(race);
                self.race_from_file = true;
                self.race_file_path = path.display().to_string();
                self.errors.clear();
                if matches!(self.activity, NewGameActivity::Invalid) {
                    self.activity = NewGameActivity::Idle;
                }
                Ok(())
            }
            Err(error) => {
                let error = NewGameError::RaceFile {
                    path: path.display().to_string(),
                    message: error.to_string(),
                };
                self.errors = vec![error.clone()];
                Err(error)
            }
        }
    }

    /// Loads the race file named by the editable path.
    ///
    /// # Errors
    ///
    /// Returns [`NewGameError::RaceFile`] for an empty path or any load failure.
    pub fn load_selected_race_file(&mut self) -> Result<(), NewGameError> {
        let path = self.race_file_path.trim().to_owned();
        if path.is_empty() {
            let error = NewGameError::RaceFile {
                path,
                message: "enter a race file path".to_owned(),
            };
            self.errors = vec![error.clone()];
            return Err(error);
        }
        self.load_race_file(Path::new(&path))
    }

    /// Clears a loaded race file so the builder race can be used again.
    pub fn clear_race_file_source(&mut self) {
        self.race_from_file = false;
        self.race = None;
        self.errors.clear();
        if matches!(self.activity, NewGameActivity::Invalid) {
            self.activity = NewGameActivity::Idle;
        }
    }

    /// Form validation errors from the last attempt.
    #[must_use]
    pub fn errors(&self) -> &[NewGameError] {
        &self.errors
    }

    /// Current activity.
    #[must_use]
    pub const fn activity(&self) -> &NewGameActivity {
        &self.activity
    }

    /// Last committed universe, retained across validation and worker failures.
    #[must_use]
    pub const fn universe(&self) -> Option<&GeneratedUniverse> {
        self.universe.as_ref()
    }

    /// Validates the form and updates the parsed seed.
    ///
    /// # Errors
    ///
    /// Returns every settings, race and seed error found.
    pub fn validate_inputs(&mut self) -> Result<(), Vec<NewGameError>> {
        let mut errors = Vec::new();
        match self.seed_text.trim().parse::<u64>() {
            Ok(seed) => self.settings.seed = seed,
            Err(_) => errors.push(NewGameError::SeedNotANumber(self.seed_text.clone())),
        }
        if let Err(settings_errors) = self.settings.validate() {
            errors.push(NewGameError::InvalidSettings(settings_errors));
        }
        match &self.race {
            Some(race) => {
                if let Err(race_errors) = validate_race(race, &ReferenceHabitability) {
                    errors.push(NewGameError::InvalidRace(race_errors));
                }
            }
            None => errors.push(NewGameError::MissingRace),
        }

        if errors.is_empty() {
            Ok(())
        } else {
            self.errors = errors.clone();
            self.activity = NewGameActivity::Invalid;
            Err(errors)
        }
    }

    /// Submits generation to the worker without clearing the current universe.
    ///
    /// # Errors
    ///
    /// Returns validation or worker-startup errors. A worker-reported failure
    /// arrives later through [`Self::apply`].
    pub fn generate(&mut self) -> Result<RequestId, Vec<NewGameError>> {
        self.validate_inputs()?;
        let race = self.race.clone().expect("validation guarantees a race");
        let request_id = RequestId::new(self.next_request);
        self.next_request = self.next_request.saturating_add(1);

        if self.worker.is_none() {
            match UniverseWorkerHandle::spawn() {
                Ok((worker, messages)) => {
                    self.worker = Some(worker);
                    self.messages = Some(messages);
                }
                Err(error) => {
                    let errors = vec![NewGameError::Worker(error.to_string())];
                    self.errors = errors.clone();
                    self.activity = NewGameActivity::Failed {
                        request_id,
                        message: error.to_string(),
                    };
                    return Err(errors);
                }
            }
        }

        let job = UniverseJob {
            request_id,
            settings: self.settings.clone(),
            race,
        };
        if let Some(worker) = &self.worker
            && let Err(error) = worker.submit(job)
        {
            let errors = vec![NewGameError::Worker(error.to_string())];
            self.errors = errors.clone();
            self.activity = NewGameActivity::Failed {
                request_id,
                message: error.to_string(),
            };
            return Err(errors);
        }

        self.errors.clear();
        self.active_request = Some(request_id);
        self.activity = NewGameActivity::Generating {
            request_id,
            progress: None,
        };
        Ok(request_id)
    }

    /// Drains completed worker messages. Called once per frame by the UI.
    pub fn drain_messages(&mut self) {
        while let Some(message) = self
            .messages
            .as_ref()
            .and_then(|messages| messages.try_recv().ok())
        {
            self.apply(message);
        }
    }

    /// Applies one worker message atomically.
    pub fn apply(&mut self, message: UniverseWorkerMessage) {
        match message {
            UniverseWorkerMessage::Progress {
                request_id,
                progress,
            } => {
                if self.active_request == Some(request_id) {
                    self.activity = NewGameActivity::Generating {
                        request_id,
                        progress: Some(progress),
                    };
                }
            }
            UniverseWorkerMessage::Completed {
                request_id,
                universe,
            } => {
                if self.active_request != Some(request_id)
                    && self
                        .last_accepted_request
                        .is_some_and(|accepted| request_id <= accepted)
                {
                    return;
                }
                self.active_request = None;
                self.last_accepted_request = Some(request_id);
                self.universe = Some(*universe);
                self.errors.clear();
                self.activity = NewGameActivity::Generated { request_id };
            }
            UniverseWorkerMessage::Failed { request_id, error } => {
                if self.active_request != Some(request_id) {
                    return;
                }
                self.active_request = None;
                let message = error.to_string();
                self.errors = vec![NewGameError::Worker(message.clone())];
                self.activity = NewGameActivity::Failed {
                    request_id,
                    message,
                };
            }
        }
    }

    /// Submits and waits for completion. Intended for tests and headless tools,
    /// not for the frame loop.
    ///
    /// # Errors
    ///
    /// Returns validation, worker-startup or worker-generation errors.
    pub fn generate_blocking(&mut self) -> Result<(), Vec<NewGameError>> {
        self.generate()?;
        loop {
            if self.active_request.is_none() {
                break;
            }
            let timeout = Duration::from_secs(20);
            match self
                .messages
                .as_ref()
                .map(|messages| messages.recv_timeout(timeout))
            {
                Some(Ok(message)) => self.apply(message),
                Some(Err(RecvTimeoutError::Timeout)) => {
                    if let Some(request) = self.active_request.take() {
                        let message = "universe generation timed out".to_owned();
                        self.errors = vec![NewGameError::Worker(message.clone())];
                        self.activity = NewGameActivity::Failed {
                            request_id: request,
                            message,
                        };
                    }
                    break;
                }
                Some(Err(RecvTimeoutError::Disconnected)) | None => {
                    if let Some(request) = self.active_request.take() {
                        let message = "universe worker stopped unexpectedly".to_owned();
                        self.errors = vec![NewGameError::Worker(message.clone())];
                        self.activity = NewGameActivity::Failed {
                            request_id: request,
                            message,
                        };
                    }
                    break;
                }
            }
        }

        match &self.activity {
            NewGameActivity::Generated { .. } => Ok(()),
            NewGameActivity::Failed { .. } => Err(self.errors.clone()),
            NewGameActivity::Invalid => Err(self.errors.clone()),
            _ => {
                let message = "universe generation did not complete".to_owned();
                self.errors = vec![NewGameError::Worker(message.clone())];
                self.activity = NewGameActivity::Failed {
                    request_id: self.active_request.unwrap_or(RequestId::new(0)),
                    message,
                };
                Err(self.errors.clone())
            }
        }
    }

    /// Short status text for the form.
    #[must_use]
    pub fn status_text(&self) -> String {
        match &self.activity {
            NewGameActivity::Idle => "Configure a new game.".to_owned(),
            NewGameActivity::Invalid => {
                "Fix the highlighted inputs; the old game is retained.".to_owned()
            }
            NewGameActivity::Generating {
                request_id,
                progress,
            } => match progress {
                Some(progress) => format!(
                    "{request_id}: {} ({}/{}).",
                    progress.stage.label(),
                    progress.completed_steps,
                    progress.total_steps
                ),
                None => format!("{request_id}: universe generation queued."),
            },
            NewGameActivity::Generated { request_id } => {
                format!("{request_id}: universe committed.")
            }
            NewGameActivity::Failed {
                request_id,
                message,
            } => format!("{request_id} failed: {message}. The previous game is retained."),
        }
    }

    /// Draws the complete new-game form.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.drain_messages();
        let mut generate = false;
        let mut load_race = false;
        let mut refresh_races = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("New Game");
            ui.label(
                "Generate a deterministic starting universe for one custom race and one or more \
                 Turindrones opponents.",
            );
            ui.separator();

            ui.heading("Universe");
            egui::Grid::new("new_game_universe")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Game name");
                    ui.text_edit_singleline(&mut self.settings.game_name);
                    ui.end_row();

                    ui.label("Seed");
                    ui.text_edit_singleline(&mut self.seed_text);
                    ui.end_row();

                    ui.label("Galaxy size");
                    egui::ComboBox::from_id_salt("galaxy_size")
                        .selected_text(self.settings.size.label())
                        .show_ui(ui, |ui| {
                            for size in stars_core::universe::GalaxySize::ALL {
                                ui.selectable_value(&mut self.settings.size, size, size.label());
                            }
                        });
                    ui.end_row();

                    ui.label("Planet density");
                    egui::ComboBox::from_id_salt("planet_density")
                        .selected_text(self.settings.density.label())
                        .show_ui(ui, |ui| {
                            for density in stars_core::universe::Density::ALL {
                                ui.selectable_value(
                                    &mut self.settings.density,
                                    density,
                                    density.label(),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("Player positions");
                    egui::ComboBox::from_id_salt("player_positions")
                        .selected_text(self.settings.player_positions.label())
                        .show_ui(ui, |ui| {
                            for position in stars_core::universe::PlayerPositions::ALL {
                                ui.selectable_value(
                                    &mut self.settings.player_positions,
                                    position,
                                    position.label(),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("Galaxy clumping");
                    ui.checkbox(&mut self.settings.clumping, "Cluster star systems");
                    ui.end_row();

                    ui.label("Turindrones opponents");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.opponent_count)
                            .range(1..=stars_core::universe::MAX_OPPONENTS),
                    );
                    ui.end_row();

                    ui.label("Difficulty");
                    egui::ComboBox::from_id_salt("difficulty")
                        .selected_text(self.settings.difficulty.label())
                        .show_ui(ui, |ui| {
                            for difficulty in stars_core::universe::Difficulty::ALL {
                                ui.selectable_value(
                                    &mut self.settings.difficulty,
                                    difficulty,
                                    difficulty.label(),
                                );
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.heading("Advanced options");
            ui.checkbox(
                &mut self.settings.options.maximum_minerals,
                "Beginner: Maximum Minerals",
            );
            ui.checkbox(
                &mut self.settings.options.slower_tech,
                "Slower Tech Advances",
            );
            ui.checkbox(
                &mut self.settings.options.accelerated_bbs,
                "Accelerated BBS Play",
            );
            ui.checkbox(
                &mut self.settings.options.no_random_events,
                "No Random Events",
            );
            ui.checkbox(
                &mut self.settings.options.computer_alliances,
                "Computer Players Form Alliances",
            );
            ui.checkbox(
                &mut self.settings.options.public_scores,
                "Public Player Scores",
            );

            ui.add_space(8.0);
            ui.heading("Victory conditions");
            for index in 0..VictoryConditionKind::ALL.len() {
                let kind = VictoryConditionKind::ALL[index];
                let condition = &mut self.settings.victory.conditions[index];
                let mut enabled = condition.enabled();
                ui.horizontal_wrapped(|ui| {
                    if ui.checkbox(&mut enabled, kind.label()).changed() {
                        condition.set_enabled(enabled);
                    }
                    let (minimum, maximum) = VictoryCondition::primary_range(kind);
                    let step = VictoryCondition::primary_step(kind);
                    if kind == VictoryConditionKind::AttainsTechnology {
                        let VictoryCondition::AttainsTechnology { level, fields, .. } = condition
                        else {
                            return;
                        };
                        let mut level_value = *level;
                        let level_min = u8::try_from(minimum).unwrap_or(8);
                        let level_max = u8::try_from(maximum).unwrap_or(26);
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Slider::new(&mut level_value, level_min..=level_max)
                                    .step_by(f64::from(step))
                                    .custom_formatter(|value, _| {
                                        format!("level {}", slider_u16(value))
                                    }),
                            )
                            .changed()
                        {
                            *level = level_value;
                        }
                        let mut field_count = *fields;
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Slider::new(&mut field_count, 2..=6)
                                    .step_by(1.0)
                                    .custom_formatter(|value, _| {
                                        format!("{} fields", slider_u16(value))
                                    }),
                            )
                            .changed()
                        {
                            *fields = field_count;
                        }
                    } else {
                        let mut value = condition.primary_threshold();
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Slider::new(&mut value, minimum..=maximum)
                                    .step_by(f64::from(step))
                                    .custom_formatter(move |raw, _| {
                                        victory_threshold_text(kind, slider_u16(raw))
                                    }),
                            )
                            .changed()
                        {
                            condition.set_primary_threshold(value);
                        }
                    }
                });
            }
            let enabled_count = self.settings.victory.enabled_count();
            if self.settings.victory.must_meet > enabled_count {
                self.settings.victory.must_meet = enabled_count;
            }
            ui.add_enabled(
                enabled_count > 0,
                egui::Slider::new(&mut self.settings.victory.must_meet, 0..=enabled_count)
                    .step_by(1.0)
                    .text("conditions that must be met"),
            );
            ui.add(
                egui::Slider::new(&mut self.settings.victory.minimum_years, 30..=500)
                    .step_by(10.0)
                    .text("minimum game years"),
            );

            ui.add_space(8.0);
            ui.heading("Human race");
            ui.horizontal(|ui| {
                ui.label("Race file");
                ui.text_edit_singleline(&mut self.race_file_path);
                if ui.button("Load").clicked() {
                    load_race = true;
                }
                if ui.button("Refresh").clicked() {
                    refresh_races = true;
                }
            });
            if !self.available_race_files.is_empty() {
                let selected = Path::new(self.race_file_path.trim())
                    .file_name()
                    .map_or_else(
                        || "Choose a saved race".to_owned(),
                        |name| name.to_string_lossy().into_owned(),
                    );
                egui::ComboBox::from_id_salt("race_file_choice")
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for path in &self.available_race_files {
                            let label = path.file_name().map_or_else(
                                || path.display().to_string(),
                                |name| name.to_string_lossy().into_owned(),
                            );
                            let value = path.display().to_string();
                            ui.selectable_value(&mut self.race_file_path, value, label);
                        }
                    });
            }
            match &self.race {
                Some(race) => {
                    ui.label(format!("{} ({})", race.name, race.primary.abbreviation()));
                    if self.race_from_file {
                        ui.horizontal(|ui| {
                            ui.label(format!("Loaded from {}.", self.race_file_path));
                            if ui.button("Clear loaded race").clicked() {
                                self.clear_race_file_source();
                            }
                        });
                    } else {
                        ui.label("Validated custom race from the race builder.");
                    }
                }
                None => {
                    ui.colored_label(
                        egui::Color32::from_rgb(210, 150, 80),
                        "No validated race. Finish the custom-race builder or load a race file.",
                    );
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                let can_generate = self.race.is_some() && !self.activity.is_generating();
                if ui
                    .add_enabled(can_generate, egui::Button::new("Generate universe"))
                    .clicked()
                {
                    generate = true;
                }
                if let Some(universe) = &self.universe {
                    ui.label(format!(
                        "Current game: {} planets, seed {}",
                        universe.planet_count(),
                        universe.settings.seed
                    ));
                }
            });

            if let NewGameActivity::Generating { progress, .. } = &self.activity
                && let Some(progress) = progress
            {
                ui.add(
                    egui::ProgressBar::new(progress.fraction())
                        .desired_width(260.0)
                        .show_percentage(),
                );
            }
            ui.label(self.status_text());

            if !self.errors.is_empty() {
                ui.add_space(4.0);
                for error in &self.errors {
                    ui.colored_label(egui::Color32::from_rgb(235, 120, 110), error.to_string());
                }
            }

            ui.add_space(8.0);
            ui.collapsing("Recovered setup notes", |ui| {
                ui.label(
                    "Planet counts use span x span / 5000 with the recovered density multiplier. \
                     Sparse subtracts a quarter, Dense adds a quarter and Packed adds another \
                     quarter over Dense; Huge is capped at 999.",
                );
                ui.label(
                    "Star coordinates use the recovered 1,000-unit border and 12-unit minimum \
                     separation. Clumping is the recovered nearest-neighbour attraction pass.",
                );
                ui.label(
                    "The seed drives a separate deterministic universe stream. The original RNG \
                     algorithm is unknown, so this is a documented remake choice.",
                );
            });
        });

        if refresh_races {
            self.refresh_race_files();
        }
        if load_race {
            let _ = self.load_selected_race_file();
        }
        if generate {
            let _ = self.generate();
        }
    }
}

/// A readable description of a generation stage, used by tests.
#[must_use]
pub fn stage_label(stage: UniverseGenerationStage) -> &'static str {
    stage.label()
}

fn victory_threshold_text(kind: VictoryConditionKind, value: u16) -> String {
    match kind {
        VictoryConditionKind::OwnsPercentPlanets => format!("{value}% of planets"),
        VictoryConditionKind::AttainsTechnology => format!("level {value}"),
        VictoryConditionKind::Score => format!("{value} points"),
        VictoryConditionKind::ExceedsNearest => format!("{value}% above nearest rival"),
        VictoryConditionKind::ProductionCapacity => format!("{value}k resource capacity"),
        VictoryConditionKind::CapitalShips => format!("{value} capital ships"),
        VictoryConditionKind::Turns => format!("{value} years"),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn slider_u16(value: f64) -> u16 {
    let rounded = value.round();
    if rounded <= 0.0 { 0 } else { rounded as u16 }
}

#[cfg(test)]
mod tests {
    use super::{NewGameActivity, NewGameState, UniverseWorkerHandle, UniverseWorkerMessage};
    use stars_core::race::{HabitabilityRange, PrimaryTrait, RaceSpec};
    use stars_core::universe::Density;

    fn race() -> RaceSpec {
        let mut race = RaceSpec::humanoids_anchor();
        race.primary = PrimaryTrait::JackOfAllTrades;
        race.habitability = [HabitabilityRange::new(15, 85); 3];
        race
    }

    #[test]
    fn validation_rejects_a_missing_race_without_touching_the_seed() {
        let mut state = NewGameState::new();
        state.set_seed(42);
        assert!(state.validate_inputs().is_err());
        assert_eq!(state.settings().seed, 42);
        assert!(matches!(state.activity(), NewGameActivity::Invalid));
    }

    #[test]
    fn invalid_size_and_density_combinations_are_reported_before_submission() {
        let mut state = NewGameState::new();
        state.set_race(Some(race()));
        state.settings_mut().density = Density::Packed;
        state.settings_mut().opponent_count = 0;
        assert!(state.generate().is_err());
    }

    #[test]
    fn worker_reports_progress_and_completion() {
        let (worker, messages) = UniverseWorkerHandle::spawn().expect("worker");
        let job = super::UniverseJob {
            request_id: crate::worker::RequestId::new(1),
            settings: {
                let mut settings = stars_core::universe::UniverseSettings::new();
                settings.opponent_count = 1;
                settings
            },
            race: race(),
        };
        worker.submit(job).expect("submit");
        let mut saw_progress = false;
        loop {
            match messages.recv_timeout(std::time::Duration::from_secs(10)) {
                Ok(UniverseWorkerMessage::Progress { .. }) => saw_progress = true,
                Ok(UniverseWorkerMessage::Completed { universe, .. }) => {
                    assert!(universe.planet_count() > 1);
                    break;
                }
                Ok(UniverseWorkerMessage::Failed { error, .. }) => panic!("{error}"),
                Err(error) => panic!("worker did not finish: {error}"),
            }
        }
        worker.shutdown();
        assert!(saw_progress);
    }
}
