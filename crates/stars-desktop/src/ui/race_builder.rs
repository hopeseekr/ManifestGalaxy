//! Desktop custom-race builder.
//!
//! The builder is a headless model plus an egui view. All point arithmetic and
//! validation come from `stars-core`; the view only edits a [`RaceDraft`] and
//! displays core results. Incomplete drafts are allowed, but [`RaceBuilderState::finish`]
//! returns only a validated [`RaceSpec`].

use eframe::egui;

use stars_core::race::{
    HABITABILITY_MIN_SPAN, HabitabilityRange, LeftoverAllocation, LesserTrait, PointTerm,
    RaceDraft, RaceEvaluation, RaceSpec, RaceValidationError, ReferenceHabitability, ResearchCost,
    validate_race,
};

use crate::race_file::{default_race_path, save_race_json};

pub use stars_core::race::spec::{
    EMBLEM_MAX, FACTORY_COST_BOUNDS, FACTORY_OPERATION_BOUNDS, FACTORY_PRODUCTION_BOUNDS,
    GROWTH_MAX, GROWTH_MIN, MINE_COST_BOUNDS, MINE_OPERATION_BOUNDS, MINE_PRODUCTION_BOUNDS,
    RESOURCE_GENERATION_BOUNDS,
};

/// Wizard page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RacePage {
    /// Step 1: name, emblem and leftover target.
    Basic,
    /// Step 2: primary trait.
    Primary,
    /// Step 3: lesser traits.
    Lesser,
    /// Step 4: habitable ranges and growth.
    Habitability,
    /// Step 5: population efficiency.
    Efficiency,
    /// Step 6: research costs and starting technology.
    Research,
    /// Final summary and validation.
    Review,
}

impl RacePage {
    /// All pages in order.
    pub const ALL: [Self; 7] = [
        Self::Basic,
        Self::Primary,
        Self::Lesser,
        Self::Habitability,
        Self::Efficiency,
        Self::Research,
        Self::Review,
    ];

    /// Page heading.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Basic => "Step 1: Basic Definition",
            Self::Primary => "Step 2: Primary Racial Trait",
            Self::Lesser => "Step 3: Lesser Racial Traits",
            Self::Habitability => "Step 4: Habitability and Growth",
            Self::Efficiency => "Step 5: Population Efficiency",
            Self::Research => "Step 6: Research Costs",
            Self::Review => "Review and Finish",
        }
    }

    /// Zero-based page index.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Basic => 0,
            Self::Primary => 1,
            Self::Lesser => 2,
            Self::Habitability => 3,
            Self::Efficiency => 4,
            Self::Research => 5,
            Self::Review => 6,
        }
    }

    /// Page after this one, saturating at Review.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Basic => Self::Primary,
            Self::Primary => Self::Lesser,
            Self::Lesser => Self::Habitability,
            Self::Habitability => Self::Efficiency,
            Self::Efficiency => Self::Research,
            Self::Research | Self::Review => Self::Review,
        }
    }

    /// Page before this one, saturating at Basic.
    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Basic | Self::Primary => Self::Basic,
            Self::Lesser => Self::Primary,
            Self::Habitability => Self::Lesser,
            Self::Efficiency => Self::Habitability,
            Self::Research => Self::Efficiency,
            Self::Review => Self::Research,
        }
    }
}

/// Headless state for the custom-race wizard.
#[derive(Debug)]
pub struct RaceBuilderState {
    draft: RaceDraft,
    page: RacePage,
    errors: Vec<RaceValidationError>,
    cached_spec: Option<RaceSpec>,
    cached_evaluation: Option<RaceEvaluation>,
    validated: Option<RaceSpec>,
    focused_text_field: Option<&'static str>,
    saved_race_path: Option<String>,
    save_error: Option<String>,
}

impl Default for RaceBuilderState {
    fn default() -> Self {
        Self::new()
    }
}

impl RaceBuilderState {
    /// Creates the builder on the first page with a neutral draft.
    #[must_use]
    pub fn new() -> Self {
        Self {
            draft: RaceDraft::neutral(),
            page: RacePage::Basic,
            errors: Vec::new(),
            cached_spec: None,
            cached_evaluation: None,
            validated: None,
            focused_text_field: None,
            saved_race_path: None,
            save_error: None,
        }
    }

    /// Read-only draft access.
    #[must_use]
    pub const fn draft(&self) -> &RaceDraft {
        &self.draft
    }

    /// Mutable draft access for tests and other UI host code.
    ///
    /// Any edit invalidates a previously validated race.
    pub fn draft_mut(&mut self) -> &mut RaceDraft {
        self.validated = None;
        self.saved_race_path = None;
        self.save_error = None;
        &mut self.draft
    }

    /// Current page.
    #[must_use]
    pub const fn page(&self) -> RacePage {
        self.page
    }

    /// Validation errors from the last finish attempt.
    #[must_use]
    pub fn errors(&self) -> &[RaceValidationError] {
        &self.errors
    }

    /// The most recently validated race, if the draft has not changed since.
    #[must_use]
    pub fn validated_race(&self) -> Option<&RaceSpec> {
        self.validated.as_ref()
    }

    /// Moves to a specific page.
    pub fn set_page(&mut self, page: RacePage) {
        self.page = page;
    }

    /// Whether the wizard may leave the current page.
    ///
    /// The primary-trait page requires an explicit selection before advancing.
    #[must_use]
    pub fn can_advance(&self) -> bool {
        self.page != RacePage::Primary || self.draft.primary.is_some()
    }

    /// Advances one page when the current page permits it.
    ///
    /// Returns `false` on the primary-trait page until a trait is selected.
    pub fn next_page(&mut self) -> bool {
        if !self.can_advance() {
            return false;
        }
        self.page = self.page.next();
        true
    }

    /// Moves back one page.
    pub fn back_page(&mut self) {
        self.page = self.page.previous();
    }

    /// Clears the whole draft, cached evaluation, errors and any validation.
    pub fn reset(&mut self) {
        self.draft = RaceDraft::neutral();
        self.page = RacePage::Basic;
        self.errors.clear();
        self.cached_spec = None;
        self.cached_evaluation = None;
        self.validated = None;
        self.focused_text_field = None;
        self.saved_race_path = None;
        self.save_error = None;
    }

    /// Evaluates the current draft, if a primary trait is selected.
    ///
    /// The result is cached by complete [`RaceSpec`] equality, so editing any
    /// field recomputes exactly once per frame.
    pub fn evaluation(&mut self) -> Option<&RaceEvaluation> {
        let spec = self.draft.to_spec().ok()?;
        if self.cached_spec.as_ref() != Some(&spec) {
            self.cached_evaluation = Some(stars_core::race::evaluate_race(
                &spec,
                &ReferenceHabitability,
            ));
            self.cached_spec = Some(spec);
        }
        self.cached_evaluation.as_ref()
    }

    /// Validates and completes the race.
    ///
    /// # Errors
    ///
    /// Returns field-specific errors for an incomplete or unaffordable race.
    pub fn finish(&mut self) -> Result<RaceSpec, Vec<RaceValidationError>> {
        let spec = match self.draft.to_spec() {
            Ok(spec) => spec,
            Err(errors) => {
                self.errors = errors.clone();
                self.validated = None;
                return Err(errors);
            }
        };

        match validate_race(&spec, &ReferenceHabitability) {
            Ok(evaluation) => {
                self.cached_spec = Some(spec.clone());
                self.cached_evaluation = Some(evaluation);
                self.errors.clear();
                self.validated = Some(spec.clone());
                Ok(spec)
            }
            Err(errors) => {
                self.errors = errors.clone();
                self.validated = None;
                Err(errors)
            }
        }
    }

    /// Validates the race and saves it to the default `races/<name>.json` path.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message when validation or file writing fails.
    pub fn save_json(&mut self) -> Result<(), String> {
        let spec = self.validate_for_save()?;
        self.save_json_to(&default_race_path(&spec))
    }

    /// Validates the race and saves it to an explicit path.
    ///
    /// # Errors
    ///
    /// Returns a human-readable message when validation or file writing fails.
    pub fn save_json_to(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.save_error = None;
        self.saved_race_path = None;
        let spec = match self.draft.to_spec() {
            Ok(spec) => spec,
            Err(errors) => return Err(self.record_save_failure(&errors)),
        };

        save_race_json(&spec, path).map_err(|error| {
            let message = error.to_string();
            self.save_error = Some(message.clone());
            message
        })?;

        self.validated = Some(spec);
        self.saved_race_path = Some(path.display().to_string());
        Ok(())
    }

    fn validate_for_save(&mut self) -> Result<RaceSpec, String> {
        match self.finish() {
            Ok(spec) => Ok(spec),
            Err(errors) => Err(self.record_save_failure(&errors)),
        }
    }

    fn record_save_failure(&mut self, errors: &[RaceValidationError]) -> String {
        let message = errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        self.save_error = Some(message.clone());
        message
    }

    /// Keyboard shortcuts that do not depend on a focused widget.
    ///
    /// Alt+Left/PageUp goes back, Alt+Right/PageDown advances, and Ctrl+R
    /// resets. Tab focus and widget activation use egui's standard keyboard
    /// handling.
    pub fn handle_shortcuts(&mut self, context: &egui::Context) {
        let (back, next, reset) = context.input(|input| {
            (
                input.key_pressed(egui::Key::PageUp)
                    || (input.modifiers.alt && input.key_pressed(egui::Key::ArrowLeft)),
                input.key_pressed(egui::Key::PageDown)
                    || (input.modifiers.alt && input.key_pressed(egui::Key::ArrowRight)),
                input.modifiers.command && input.key_pressed(egui::Key::R),
            )
        });

        if back {
            self.back_page();
        }
        if next {
            self.next_page();
        }
        if reset {
            self.reset();
        }
    }

    /// Draws the builder.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.handle_shortcuts(ui.ctx());
        self.clear_stale_validation();

        self.show_header(ui);
        ui.separator();

        // Reserve room for the two-row footer at the enlarged application text
        // size so navigation is never pushed below the visible window.
        let footer_reserve = 150.0;
        let scroll_height = (ui.available_height() - footer_reserve).max(80.0);
        egui::ScrollArea::vertical()
            .max_height(scroll_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.page {
                    RacePage::Basic => self.show_basic(ui),
                    RacePage::Primary => self.show_primary(ui),
                    RacePage::Lesser => self.show_lesser(ui),
                    RacePage::Habitability => self.show_habitability(ui),
                    RacePage::Efficiency => self.show_efficiency(ui),
                    RacePage::Research => self.show_research(ui),
                    RacePage::Review => self.show_review(ui),
                }
                self.show_page_errors(ui);
            });

        ui.separator();
        self.show_footer(ui);
    }

    fn clear_stale_validation(&mut self) {
        let current = self.draft.to_spec().ok();
        if let Some(validated) = &self.validated
            && current.as_ref() != Some(validated)
        {
            self.validated = None;
        }
    }

    fn show_header(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Custom Race Builder");
            ui.label(format!("{} of 6", self.page.index().min(5) + 1));
        });
        ui.label(egui::RichText::new(self.page.title()).strong());
        if self.page == RacePage::Review {
            ui.label("Review the complete race before finishing.");
        }
    }

    fn show_footer(&mut self, ui: &mut egui::Ui) {
        let points = self
            .evaluation()
            .map(|evaluation| (evaluation.displayed, evaluation.is_affordable()));

        ui.horizontal_wrapped(|ui| {
            match points {
                Some((displayed, affordable)) => {
                    let color = if affordable {
                        egui::Color32::from_rgb(120, 210, 140)
                    } else {
                        egui::Color32::from_rgb(240, 130, 130)
                    };
                    ui.label(
                        egui::RichText::new(format!("Advantage points: {displayed}"))
                            .strong()
                            .color(color),
                    );
                }
                None => {
                    ui.label(
                        egui::RichText::new("Select a primary trait to calculate points.")
                            .italics(),
                    );
                }
            }

            ui.add_space(12.0);
            if ui.button("Reset").clicked() {
                self.reset();
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.page != RacePage::Basic,
                    egui::Button::new("Back").min_size(egui::vec2(90.0, 28.0)),
                )
                .clicked()
            {
                self.back_page();
            }

            if self.page == RacePage::Review {
                if ui
                    .add(egui::Button::new("Finish").min_size(egui::vec2(110.0, 28.0)))
                    .clicked()
                {
                    let _ = self.finish();
                }
            } else {
                let next_label = format!("Next: {}", self.page.next().title());
                if ui
                    .add_enabled(
                        self.can_advance(),
                        egui::Button::new(next_label)
                            .min_size(egui::vec2(190.0, 28.0))
                            .fill(ui.visuals().selection.bg_fill),
                    )
                    .clicked()
                {
                    self.next_page();
                }
            }

            ui.label(
                egui::RichText::new("PageUp/PageDown or Alt+Left/Right move between steps.").weak(),
            );
        });

        if let Some(validated) = &self.validated {
            ui.label(
                egui::RichText::new(format!(
                    "Validated: {} ({})",
                    validated.name,
                    validated.primary.name()
                ))
                .color(egui::Color32::from_rgb(120, 210, 140)),
            );
        }
    }

    fn show_basic(&mut self, ui: &mut egui::Ui) {
        ui.label("Race name");
        text_field(
            ui,
            "name",
            &mut self.draft.name,
            &mut self.focused_text_field,
            260.0,
            false,
        );
        ui.label("Plural name");
        text_field(
            ui,
            "plural_name",
            &mut self.draft.plural_name,
            &mut self.focused_text_field,
            260.0,
            false,
        );
        ui.label("Race-file password");
        text_field(
            ui,
            "password",
            &mut self.draft.password,
            &mut self.focused_text_field,
            260.0,
            true,
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Emblem");
            ui.add(
                egui::DragValue::new(&mut self.draft.emblem)
                    .range(0..=EMBLEM_MAX)
                    .speed(1),
            );
        });

        ui.add_space(8.0);
        ui.label("Spend leftover advantage points on");
        for choice in LeftoverAllocation::ALL {
            ui.radio_value(&mut self.draft.leftover, choice, choice.label());
        }
    }

    fn show_primary(&mut self, ui: &mut egui::Ui) {
        ui.label("Exactly one primary trait is required.");
        if self.draft.primary.is_none() {
            ui.label(
                egui::RichText::new("Select a primary trait to continue.")
                    .color(egui::Color32::from_rgb(235, 190, 110)),
            );
        }
        for trait_value in stars_core::race::PrimaryTrait::ALL {
            let label = format!("{} ({})", trait_value.name(), trait_value.abbreviation());
            ui.radio_value(&mut self.draft.primary, Some(trait_value), label);
        }
    }

    fn show_lesser(&mut self, ui: &mut egui::Ui) {
        ui.label("Zero or more lesser traits are allowed.");
        for trait_value in LesserTrait::ALL {
            let mut selected = self.draft.lesser.contains(&trait_value);
            let label = format!("{} ({})", trait_value.name(), trait_value.abbreviation());
            if ui.checkbox(&mut selected, label).changed() {
                if selected {
                    self.draft.lesser.insert(trait_value);
                } else {
                    self.draft.lesser.remove(&trait_value);
                }
            }
        }

        if self.draft.lesser.len() > 4 {
            ui.label(
                egui::RichText::new("More than four lesser traits triggers a point penalty.")
                    .color(egui::Color32::from_rgb(235, 190, 110)),
            );
        }
    }

    fn show_habitability(&mut self, ui: &mut egui::Ui) {
        for (axis, name) in ["Gravity", "Temperature", "Radiation"].iter().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(*name).strong());
                    ui.checkbox(&mut self.draft.habitability[axis].immune, "Immune");
                });

                let band = &mut self.draft.habitability[axis];
                ui.add_enabled_ui(!band.immune, |ui| {
                    ui.horizontal(|ui| {
                        ui.monospace(format!("{:>3}", band.min));
                        range_slider(ui, axis, &mut band.min, &mut band.max);
                        ui.monospace(format!("{:>3}", band.max));
                    });
                    ui.label(format!(
                        "Range {}..{}  |  midpoint {}  |  width {}",
                        band.min,
                        band.max,
                        band.ideal(),
                        band.max - band.min
                    ));
                });

                if band.immune {
                    ui.label("Ideal: immune");
                }
            });
        }

        ui.add_space(8.0);
        ui.add(
            egui::Slider::new(&mut self.draft.growth_rate, GROWTH_MIN..=GROWTH_MAX)
                .text("Ideal growth rate"),
        );
    }

    fn show_efficiency(&mut self, ui: &mut egui::Ui) {
        let alternate_reality = self
            .draft
            .primary
            .is_some_and(|primary| !primary.has_planetary_installations());

        if alternate_reality {
            ui.label(egui::RichText::new("Efficiency coefficient").strong());
            ui.add(
                egui::Slider::new(
                    &mut self.draft.efficiency.resource_generation,
                    RESOURCE_GENERATION_BOUNDS.min..=RESOURCE_GENERATION_BOUNDS.max,
                )
                .text("Colonists per resource"),
            );
            ui.label("Resource output grows with Energy technology.");
            return;
        }

        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.resource_generation,
                RESOURCE_GENERATION_BOUNDS.min..=RESOURCE_GENERATION_BOUNDS.max,
            )
            .text("Colonists per resource"),
        );
        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.factory_production,
                FACTORY_PRODUCTION_BOUNDS.min..=FACTORY_PRODUCTION_BOUNDS.max,
            )
            .text("Factories produced per 10,000 colonists"),
        );
        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.factory_cost,
                FACTORY_COST_BOUNDS.min..=FACTORY_COST_BOUNDS.max,
            )
            .text("Factory cost"),
        );
        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.factory_operation,
                FACTORY_OPERATION_BOUNDS.min..=FACTORY_OPERATION_BOUNDS.max,
            )
            .text("Factories operated per 10,000 colonists"),
        );
        ui.checkbox(&mut self.draft.cheap_factories, "Cheap factory materials");

        ui.separator();
        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.mine_production,
                MINE_PRODUCTION_BOUNDS.min..=MINE_PRODUCTION_BOUNDS.max,
            )
            .text("Mines produced per 10,000 colonists"),
        );
        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.mine_cost,
                MINE_COST_BOUNDS.min..=MINE_COST_BOUNDS.max,
            )
            .text("Mine cost"),
        );
        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.mine_operation,
                MINE_OPERATION_BOUNDS.min..=MINE_OPERATION_BOUNDS.max,
            )
            .text("Mines operated per 10,000 colonists"),
        );
    }

    fn show_research(&mut self, ui: &mut egui::Ui) {
        ui.label("Each field costs 50% less, standard, or 75% extra.");
        for (index, field) in stars_core::race::spec::RESEARCH_FIELD_NAMES
            .iter()
            .enumerate()
        {
            ui.group(|ui| {
                ui.label(egui::RichText::new(*field).strong());
                ui.horizontal_wrapped(|ui| {
                    for choice in [
                        ResearchCost::Extra,
                        ResearchCost::Standard,
                        ResearchCost::Cheaper,
                    ] {
                        ui.radio_value(
                            &mut self.draft.research.costs[index],
                            choice,
                            choice.label(),
                        );
                    }
                });
            });
        }
        ui.checkbox(
            &mut self.draft.research.tech3,
            "All fields start at technology 3",
        );
    }

    fn show_review(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("race_review_summary")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Name");
                ui.label(format!("{} / {}", self.draft.name, self.draft.plural_name));
                ui.end_row();
                ui.label("Primary trait");
                ui.label(
                    self.draft
                        .primary
                        .map_or("<not selected>", |trait_value| trait_value.name()),
                );
                ui.end_row();
                ui.label("Lesser traits");
                let lesser = if self.draft.lesser.is_empty() {
                    "<none>".to_owned()
                } else {
                    self.draft
                        .lesser
                        .iter()
                        .map(|trait_value| trait_value.abbreviation())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                ui.label(lesser);
                ui.end_row();
                ui.label("Growth");
                ui.label(format!("{}%", self.draft.growth_rate));
                ui.end_row();
                ui.label("Habitability");
                ui.label(format!(
                    "{} / {} / {}",
                    band_summary(self.draft.habitability[0]),
                    band_summary(self.draft.habitability[1]),
                    band_summary(self.draft.habitability[2])
                ));
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Advantage-point breakdown").strong());
        egui::Grid::new("race_review_points")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Term");
                ui.label("Raw");
                ui.end_row();
                let terms = self
                    .evaluation()
                    .map_or_else(Vec::new, |evaluation| evaluation.terms.clone());
                for term in terms.iter().filter(|term| term.raw != 0) {
                    ui.label(point_term_label(term));
                    ui.label(signed_i64(term.raw));
                    ui.end_row();
                }
                let (raw, displayed) = self
                    .evaluation()
                    .map_or((0, 0), |evaluation| (evaluation.raw, evaluation.displayed));
                ui.label(egui::RichText::new("Total").strong());
                ui.label(
                    egui::RichText::new(format!("{} raw = {} AP", signed_i64(raw), displayed))
                        .strong(),
                );
                ui.end_row();
            });

        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui.button("Save race as JSON").clicked() {
                let _ = self.save_json();
            }
            if let Some(path) = &self.saved_race_path {
                ui.label(
                    egui::RichText::new(format!("Saved to {path}"))
                        .color(egui::Color32::from_rgb(120, 210, 140)),
                );
            }
        });
        if let Some(error) = &self.save_error {
            ui.label(
                egui::RichText::new(format!("Could not save race: {error}"))
                    .color(egui::Color32::from_rgb(240, 130, 130)),
            );
        }

        if let Some(validated) = &self.validated {
            ui.label(
                egui::RichText::new(format!(
                    "Race \"{}\" is validated and ready for the new-game flow ({} AP).",
                    validated.name,
                    self.cached_evaluation
                        .as_ref()
                        .map_or(0, |evaluation| evaluation.displayed)
                ))
                .color(egui::Color32::from_rgb(120, 210, 140)),
            );
        }
    }

    fn show_page_errors(&self, ui: &mut egui::Ui) {
        if self.errors.is_empty() {
            return;
        }
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Validation errors")
                .strong()
                .color(egui::Color32::from_rgb(240, 130, 130)),
        );
        for error in &self.errors {
            ui.label(format!("{}: {error}", error.field_key()));
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RangeHandle {
    #[default]
    Minimum,
    Maximum,
}

fn range_slider(ui: &mut egui::Ui, axis: usize, min: &mut u8, max: &mut u8) -> egui::Response {
    let id = ui.make_persistent_id(("habit_range", axis));
    let width = ui.available_width().clamp(220.0, 520.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 38.0), egui::Sense::click_and_drag());

    let enabled = ui.is_enabled();
    let track = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 12.0, rect.center().y - 4.0),
        egui::pos2(rect.right() - 12.0, rect.center().y + 4.0),
    );
    let handle_key = id.with("active_handle");
    let mut active = ui
        .data(|data| data.get_temp::<RangeHandle>(handle_key))
        .unwrap_or_default();

    let value_at = |x: f32| -> u8 {
        let fraction = ((x - track.left()) / track.width()).clamp(0.0, 1.0);
        (fraction * 100.0).round() as u8
    };

    if enabled {
        if response.drag_started() || response.clicked() {
            if let Some(position) = response.interact_pointer_pos() {
                let minimum_x = value_x(track, *min);
                let maximum_x = value_x(track, *max);
                active = if (position.x - minimum_x).abs() <= (position.x - maximum_x).abs() {
                    RangeHandle::Minimum
                } else {
                    RangeHandle::Maximum
                };
                ui.data_mut(|data| data.insert_temp(handle_key, active));
                set_range_handle(min, max, active, value_at(position.x));
            }
        } else if response.dragged()
            && let Some(position) = response.interact_pointer_pos()
        {
            active = ui
                .data(|data| data.get_temp::<RangeHandle>(handle_key))
                .unwrap_or(active);
            set_range_handle(min, max, active, value_at(position.x));
        }

        if response.has_focus() {
            let mut delta = 0_i16;
            ui.input_mut(|input| {
                if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                {
                    delta -= 1;
                }
                if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                {
                    delta += 1;
                }
            });
            if delta != 0 {
                active = ui
                    .data(|data| data.get_temp::<RangeHandle>(handle_key))
                    .unwrap_or(active);
                let current = match active {
                    RangeHandle::Minimum => *min,
                    RangeHandle::Maximum => *max,
                };
                let next = (i16::from(current) + delta).clamp(0, 100) as u8;
                set_range_handle(min, max, active, next);
            }
        }
    }

    let visuals = ui.visuals();
    let painter = ui.painter_at(rect);
    let track_color = if enabled {
        visuals.widgets.inactive.bg_fill
    } else {
        visuals.widgets.noninteractive.bg_fill
    };
    let filled_color = if enabled {
        visuals.selection.bg_fill
    } else {
        visuals.widgets.noninteractive.bg_stroke.color
    };
    painter.rect_filled(track, 4.0, track_color);

    let minimum_x = value_x(track, *min);
    let maximum_x = value_x(track, *max);
    let filled = egui::Rect::from_min_max(
        egui::pos2(minimum_x, track.top()),
        egui::pos2(maximum_x, track.bottom()),
    );
    painter.rect_filled(filled, 4.0, filled_color);

    for (value, is_active) in [
        (*min, active == RangeHandle::Minimum),
        (*max, active == RangeHandle::Maximum),
    ] {
        let x = value_x(track, value);
        let center = egui::pos2(x, rect.center().y);
        let handle_color = if !enabled {
            visuals.widgets.noninteractive.fg_stroke.color
        } else if is_active || response.has_focus() {
            visuals.widgets.active.bg_fill
        } else {
            visuals.widgets.inactive.bg_fill
        };
        painter.circle_filled(center, 9.0, handle_color);
        painter.circle_stroke(
            center,
            9.0,
            egui::Stroke::new(1.5, visuals.widgets.active.fg_stroke.color),
        );
    }

    if response.has_focus() {
        painter.rect_stroke(
            rect.shrink(2.0),
            4.0,
            egui::Stroke::new(1.0, visuals.selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }

    response
}

fn value_x(track: egui::Rect, value: u8) -> f32 {
    track.left() + track.width() * (f32::from(value) / 100.0)
}

fn set_range_handle(min: &mut u8, max: &mut u8, handle: RangeHandle, value: u8) {
    match handle {
        RangeHandle::Minimum => {
            let upper = max.saturating_sub(HABITABILITY_MIN_SPAN);
            *min = value.min(upper);
        }
        RangeHandle::Maximum => {
            let lower = min.saturating_add(HABITABILITY_MIN_SPAN).min(100);
            *max = value.max(lower).min(100);
        }
    }
}

fn field_id(name: &'static str) -> egui::Id {
    egui::Id::new(("race_builder_text", name))
}

fn text_field(
    ui: &mut egui::Ui,
    id_name: &'static str,
    text: &mut String,
    focused_field: &mut Option<&'static str>,
    desired_width: f32,
    password: bool,
) -> egui::Response {
    let id = field_id(id_name);
    let mut edit = egui::TextEdit::singleline(text)
        .id(id)
        .desired_width(desired_width);
    if password {
        edit = edit.password(true);
    }
    let response = ui.add(edit);
    if response.has_focus() {
        if *focused_field != Some(id_name) {
            select_all_text(ui, id, text);
        }
        *focused_field = Some(id_name);
    } else if *focused_field == Some(id_name) {
        *focused_field = None;
    }
    response
}

fn select_all_text(ui: &egui::Ui, id: egui::Id, text: &str) {
    let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::default(),
            egui::text::CCursor::new(text.chars().count()),
        )));
    state.store(ui.ctx(), id);
}

fn band_summary(band: HabitabilityRange) -> String {
    if band.immune {
        "immune".to_owned()
    } else {
        format!("{}..{}", band.min, band.max)
    }
}

fn point_term_label(term: &PointTerm) -> &'static str {
    term.label
}

fn signed_i64(value: i64) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{RaceBuilderState, RacePage, field_id};
    use eframe::egui;
    use stars_core::race::{
        LesserTrait, PrimaryTrait, RaceDraft, RaceSpec, ReferenceHabitability, validate_race,
    };

    #[test]
    fn range_handle_enforces_the_five_point_gap() {
        let mut min = 20_u8;
        let mut max = 30_u8;
        super::set_range_handle(&mut min, &mut max, super::RangeHandle::Minimum, 29);
        assert_eq!((min, max), (25, 30));

        super::set_range_handle(&mut min, &mut max, super::RangeHandle::Maximum, 20);
        assert_eq!((min, max), (25, 30));

        super::set_range_handle(&mut min, &mut max, super::RangeHandle::Maximum, 90);
        assert_eq!((min, max), (25, 90));
    }

    #[test]
    fn focusing_a_text_field_selects_all_of_its_text() {
        let context = egui::Context::default();
        let mut builder = RaceBuilderState::new();
        builder.draft_mut().name = "Alpha".to_owned();
        let id = field_id("name");
        context.memory_mut(|memory| memory.request_focus(id));

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            ..Default::default()
        };
        let output = context.run_ui(input.clone(), |ui| builder.draw(ui));
        output.drop_without_applying_deltas();
        // Focus requested before the first pass becomes observable on the next
        // pass, so render twice before inspecting the selection.
        let output = context.run_ui(input, |ui| builder.draw(ui));
        output.drop_without_applying_deltas();

        let state = egui::TextEdit::load_state(&context, id).expect("text edit state");
        let range = state.cursor.char_range().expect("selection");
        assert_eq!(range.as_sorted_char_range().start.0, 0);
        assert_eq!(range.as_sorted_char_range().end.0, 5);
    }

    #[test]
    fn save_json_to_writes_a_validated_race() {
        let mut builder = RaceBuilderState::new();
        *builder.draft_mut() = RaceDraft::from_spec(&RaceSpec::humanoids_anchor());
        builder.draft_mut().name = "Saved Race".to_owned();
        let path = std::env::temp_dir().join(format!(
            "stars-race-builder-save-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        builder.save_json_to(&path).expect("save");
        let loaded = crate::race_file::load_race_json(&path).expect("load");
        std::fs::remove_file(&path).expect("remove fixture");

        assert_eq!(loaded.name, "Saved Race");
        assert_eq!(
            builder.saved_race_path.as_deref(),
            Some(path.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn primary_trait_must_be_selected_before_advancing() {
        let mut builder = RaceBuilderState::new();
        builder.set_page(RacePage::Primary);
        assert!(!builder.can_advance());
        assert!(!builder.next_page());
        assert_eq!(builder.page(), RacePage::Primary);

        builder.draft_mut().primary = Some(PrimaryTrait::JackOfAllTrades);
        assert!(builder.can_advance());
        assert!(builder.next_page());
        assert_eq!(builder.page(), RacePage::Lesser);
    }

    #[test]
    fn draft_edits_are_kept_when_stepping_backward() {
        let mut builder = RaceBuilderState::new();
        builder.set_page(RacePage::Primary);
        builder.draft_mut().primary = Some(PrimaryTrait::SpaceDemolition);
        builder.next_page();
        builder
            .draft_mut()
            .lesser
            .insert(LesserTrait::TotalTerraforming);
        builder.next_page();

        builder.back_page();
        assert_eq!(builder.page(), RacePage::Lesser);
        assert!(
            builder
                .draft()
                .lesser
                .contains(&LesserTrait::TotalTerraforming)
        );
        assert_eq!(builder.draft().primary, Some(PrimaryTrait::SpaceDemolition));
    }

    #[test]
    fn reset_returns_every_field_to_the_neutral_draft() {
        let mut builder = RaceBuilderState::new();
        builder.draft_mut().name = "Changed".to_owned();
        builder.draft_mut().primary = Some(PrimaryTrait::WarMonger);
        builder
            .draft_mut()
            .lesser
            .insert(LesserTrait::MineralAlchemy);
        builder.set_page(RacePage::Research);
        builder.reset();

        assert_eq!(builder.page(), RacePage::Basic);
        assert_eq!(builder.draft(), &stars_core::race::RaceDraft::neutral());
        assert!(builder.validated_race().is_none());
    }

    #[test]
    fn builder_evaluation_agrees_with_core() {
        let mut builder = RaceBuilderState::new();
        builder.draft_mut().primary = Some(PrimaryTrait::PacketPhysics);
        builder
            .draft_mut()
            .lesser
            .insert(LesserTrait::NoAdvancedScanners);
        builder.draft_mut().growth_rate = 7;
        let spec = builder.draft().to_spec().expect("primary selected");
        let core = validate_race(&spec, &ReferenceHabitability).expect("affordable");
        let ui = builder.evaluation().expect("evaluation");
        assert_eq!(ui.displayed, core.displayed);
        assert_eq!(ui.raw, core.raw);
    }
}
