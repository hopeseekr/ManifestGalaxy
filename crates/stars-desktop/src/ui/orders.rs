//! Draft-order editing through the core command contract.
//!
//! The draft holds no authoritative state. It builds an [`OrderBatch`] from the
//! currently displayed player view and asks [`GameSession`] to validate it.
//! Unavailable gameplay orders remain disabled rather than being accepted as
//! silent no-ops.

use eframe::egui;

use stars_core::model::{BatchId, FleetId, PlayerId};
use stars_core::orders::{Command, CommandAction, CommandId, OrderBatch, OrderError};
use stars_core::turn::GameSession;
use stars_core::view::PlayerView;

use super::map::Selection;

/// Result of the last draft validation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DraftStatus {
    /// No validation has run for the current draft.
    #[default]
    NotValidated,
    /// There are no pending changes.
    NoChanges,
    /// Core accepted the draft.
    Valid,
    /// Core rejected the draft with typed errors.
    Invalid,
}

/// A pending administrative order draft.
#[derive(Clone, Debug)]
pub struct OrderDraft {
    ready: bool,
    rename_fleet: Option<FleetId>,
    rename_text: String,
    player: Option<PlayerId>,
    turn: Option<u32>,
    next_command: u64,
    errors: Vec<OrderError>,
    status: DraftStatus,
    synced_selection: Selection,
}

impl Default for OrderDraft {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderDraft {
    /// Creates an empty draft.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ready: false,
            rename_fleet: None,
            rename_text: String::new(),
            player: None,
            turn: None,
            next_command: 1,
            errors: Vec::new(),
            status: DraftStatus::NotValidated,
            synced_selection: Selection::None,
        }
    }

    /// Pending readiness value.
    #[must_use]
    pub const fn ready(&self) -> bool {
        self.ready
    }

    /// Sets the pending readiness value and invalidates prior validation.
    pub fn set_ready(&mut self, ready: bool) {
        if self.ready != ready {
            self.ready = ready;
            self.clear_validation();
        }
    }

    /// Fleet targeted by the pending rename, if any.
    #[must_use]
    pub const fn rename_fleet(&self) -> Option<FleetId> {
        self.rename_fleet
    }

    /// Pending fleet name.
    #[must_use]
    pub fn rename_text(&self) -> &str {
        &self.rename_text
    }

    /// Mutable fleet name for the UI.
    pub fn rename_text_mut(&mut self) -> &mut String {
        self.clear_validation();
        &mut self.rename_text
    }

    /// Core validation errors from the last attempt.
    #[must_use]
    pub fn errors(&self) -> &[OrderError] {
        &self.errors
    }

    /// Last validation result.
    #[must_use]
    pub const fn status(&self) -> DraftStatus {
        self.status
    }

    /// Synchronises the draft with a newly shown view and linked selection.
    ///
    /// A new player or turn resets the draft. Selecting another owned fleet
    /// replaces the rename target with that fleet's current visible name.
    pub fn sync(&mut self, view: &PlayerView, selection: Selection) {
        if self.player != Some(view.player) || self.turn != Some(view.turn) {
            self.player = Some(view.player);
            self.turn = Some(view.turn);
            self.ready = view.ready;
            self.rename_fleet = None;
            self.rename_text.clear();
            self.synced_selection = Selection::None;
            self.clear_validation();
        }

        if selection != self.synced_selection {
            self.synced_selection = selection;
            if let Selection::Fleet(id) = selection
                && let Some(fleet) = view
                    .fleets
                    .iter()
                    .find(|fleet| fleet.id == id && fleet.owner == view.player)
            {
                self.rename_fleet = Some(id);
                self.rename_text.clone_from(&fleet.name);
                self.clear_validation();
            }
        }

        if let Some(id) = self.rename_fleet
            && !view
                .fleets
                .iter()
                .any(|fleet| fleet.id == id && fleet.owner == view.player)
        {
            self.rename_fleet = None;
            self.rename_text.clear();
            self.clear_validation();
        }
    }

    /// Builds the current batch without mutating authoritative state.
    ///
    /// Command identifiers are stable for the life of the draft so validation
    /// and a subsequent submission describe the same ordered actions.
    #[must_use]
    pub fn build_batch(&self, view: &PlayerView) -> OrderBatch {
        let mut batch = OrderBatch::new(
            BatchId::from_raw(u64::from(view.turn).saturating_add(1)),
            view.player,
            view.turn,
        );
        let mut command = self.next_command;
        if self.ready != view.ready {
            batch = batch.with_command(Command::new(
                CommandId::from_raw(command),
                CommandAction::SetReady { ready: self.ready },
            ));
            command = command.saturating_add(1);
        }
        if let Some(fleet) = self.rename_fleet
            && !self.rename_text.trim().is_empty()
        {
            let visible_name = view
                .fleets
                .iter()
                .find(|visible| visible.id == fleet && visible.owner == view.player)
                .map(|visible| visible.name.as_str());
            if visible_name != Some(self.rename_text.as_str()) {
                batch = batch.with_command(Command::new(
                    CommandId::from_raw(command),
                    CommandAction::RenameFleet {
                        fleet,
                        name: self.rename_text.clone(),
                    },
                ));
            }
        }
        batch
    }

    /// Whether the draft contains at least one command.
    #[must_use]
    pub fn has_changes(&self, view: &PlayerView) -> bool {
        !self.build_batch(view).commands.is_empty()
    }

    /// Validates the draft with core without committing it.
    ///
    /// # Errors
    ///
    /// Returns the core [`OrderError`] list. The same errors are retained for
    /// display. Unknown or unauthorized fleets are rejected by core rather than
    /// silently filtered by the widget.
    pub fn validate(
        &mut self,
        view: &PlayerView,
        session: &GameSession,
    ) -> Result<(), Vec<OrderError>> {
        let batch = self.build_batch(view);
        if batch.commands.is_empty() {
            self.errors.clear();
            self.status = DraftStatus::NoChanges;
            return Ok(());
        }
        match session.validate_orders(&batch) {
            Ok(()) => {
                self.errors.clear();
                self.status = DraftStatus::Valid;
                Ok(())
            }
            Err(error) => {
                self.errors = vec![error];
                self.status = DraftStatus::Invalid;
                Err(self.errors.clone())
            }
        }
    }

    /// Draws the draft panel. Returns `true` when the user asks to validate.
    pub fn draw(&mut self, ui: &mut egui::Ui, view: &PlayerView, selection: Selection) -> bool {
        self.sync(view, selection);
        let mut validate = false;

        ui.heading("Draft orders");
        ui.label(format!(
            "Turn {} | year {} | {}",
            view.turn,
            view.year,
            if view.ready { "ready" } else { "not ready" }
        ));

        let mut ready = self.ready;
        if ui
            .checkbox(&mut ready, "Ready for turn generation")
            .changed()
        {
            self.set_ready(ready);
        }

        ui.add_space(4.0);
        ui.label("Selected fleet");
        match selection {
            Selection::Fleet(id) => {
                if let Some(fleet) = view.fleets.iter().find(|fleet| fleet.id == id) {
                    if fleet.owner == view.player {
                        if self.rename_fleet != Some(id) {
                            self.rename_fleet = Some(id);
                            self.rename_text.clone_from(&fleet.name);
                            self.clear_validation();
                        }
                        ui.horizontal(|ui| {
                            ui.label("Name");
                            if ui.text_edit_singleline(&mut self.rename_text).changed() {
                                self.clear_validation();
                            }
                        });
                    } else {
                        ui.colored_label(
                            egui::Color32::from_rgb(210, 150, 80),
                            "Another player's fleet cannot be renamed.",
                        );
                    }
                } else {
                    ui.label("The selected fleet is no longer visible.");
                }
            }
            Selection::Planet(_) | Selection::None => {
                ui.label("Select one of your fleets to edit its name.");
            }
        }

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled(false, egui::Button::new("Set waypoints"))
                .on_disabled_hover_text("Fleet movement orders arrive in PR10.");
            ui.add_enabled(false, egui::Button::new("Load cargo"))
                .on_disabled_hover_text("Cargo orders arrive in PR10.");
            ui.add_enabled(false, egui::Button::new("Colonize"))
                .on_disabled_hover_text("Colonization orders arrive in PR10.");
        });

        ui.add_space(4.0);
        if ui
            .add_enabled(self.has_changes(view), egui::Button::new("Validate draft"))
            .clicked()
        {
            validate = true;
        }

        match self.status {
            DraftStatus::NotValidated => {}
            DraftStatus::NoChanges => {
                ui.label("No pending changes.");
            }
            DraftStatus::Valid => {
                ui.colored_label(
                    egui::Color32::from_rgb(120, 206, 150),
                    "Core accepted the draft. Submission remains PR10 work.",
                );
            }
            DraftStatus::Invalid => {
                for error in &self.errors {
                    ui.colored_label(egui::Color32::from_rgb(235, 120, 110), error.to_string());
                }
            }
        }

        validate
    }

    fn clear_validation(&mut self) {
        self.errors.clear();
        self.status = DraftStatus::NotValidated;
    }
}
