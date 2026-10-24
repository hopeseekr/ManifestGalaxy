//! Desktop custom-race builder.
//!
//! The builder is a headless model plus an egui view. All point arithmetic and
//! validation come from `stars-core`; the view only edits a [`RaceDraft`] and
//! displays core results. Incomplete drafts are allowed, but [`RaceBuilderState::finish`]
//! returns only a validated [`RaceSpec`].

use eframe::egui;

use stars_core::race::{
    HabitabilityRange, LeftoverAllocation, LesserTrait, PointTerm, RaceDraft, RaceEvaluation,
    RaceField, RaceSpec, RaceValidationError, ReferenceHabitability, ResearchCost, validate_race,
};

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

    /// Advances one page.
    pub fn next_page(&mut self) {
        self.page = self.page.next();
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

        egui::ScrollArea::vertical()
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
            ui.label(
                "Review the complete race. Finish accepts only a race that passes core validation.",
            );
        }
        ui.label(
            egui::RichText::new(
                "Advantage points are recalculated from the core formula as fields change.",
            )
            .weak(),
        );
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
                    )
                    .on_hover_text("Displayed balance. A completed race must be zero or greater.");
                }
                None => {
                    ui.label(
                        egui::RichText::new("Select a primary trait to calculate points.")
                            .italics(),
                    );
                }
            }

            ui.add_space(12.0);
            if ui.button("Reset").on_hover_text("Ctrl+R").clicked() {
                self.reset();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.page == RacePage::Review {
                    if ui.button("Finish").clicked() {
                        let _ = self.finish();
                    }
                } else if ui.button("Next").on_hover_text("PageDown").clicked() {
                    self.next_page();
                }
                if ui
                    .add_enabled(self.page != RacePage::Basic, egui::Button::new("Back"))
                    .on_hover_text("PageUp")
                    .clicked()
                {
                    self.back_page();
                }
            });
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
        ui.add(
            egui::TextEdit::singleline(&mut self.draft.name)
                .desired_width(260.0)
                .hint_text("Required"),
        )
        .on_hover_text("Stored in the recovered 32-byte name field.");
        ui.label("Plural name");
        ui.add(
            egui::TextEdit::singleline(&mut self.draft.plural_name)
                .desired_width(260.0)
                .hint_text("Required"),
        )
        .on_hover_text("Used for multi-planet empire labels.");
        ui.label("Race-file password");
        ui.add(
            egui::TextEdit::singleline(&mut self.draft.password)
                .desired_width(260.0)
                .password(true),
        )
        .on_hover_text(
            "Local race-file field from the recovered wizard. It is not a security boundary and is never logged.",
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Emblem");
            ui.add(
                egui::DragValue::new(&mut self.draft.emblem)
                    .range(0..=EMBLEM_MAX)
                    .speed(1),
            )
            .on_hover_text("Emblem index from the recovered five-bit portrait field.");
        });

        ui.add_space(8.0);
        ui.label("Spend leftover advantage points on");
        for choice in LeftoverAllocation::ALL {
            ui.radio_value(&mut self.draft.leftover, choice, choice.label())
                .on_hover_text(
                    "PR6 owns the conversion arithmetic; PR5 records the selection only.",
                );
        }
    }

    fn show_primary(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Exactly one primary trait is required. Selecting a trait does not claim its later \
             runtime effect is implemented.",
        );
        for trait_value in stars_core::race::PrimaryTrait::ALL {
            let cost = trait_value.point_cost_raw();
            let label = format!(
                "{} ({})  [{} raw]",
                trait_value.name(),
                trait_value.abbreviation(),
                signed(cost)
            );
            ui.radio_value(&mut self.draft.primary, Some(trait_value), label)
                .on_hover_text(format!(
                    "{} costs are recovered from the primary trait table. Runtime effect: {}.",
                    trait_value.name(),
                    trait_value.effect_owner_prs().join(", ")
                ));
        }
    }

    fn show_lesser(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Zero or more lesser traits are allowed. Exclusions were not recovered, so every \
             priced combination is selectable.",
        );
        for trait_value in LesserTrait::ALL {
            let mut selected = self.draft.lesser.contains(&trait_value);
            let cost = trait_value.cost();
            let label = format!(
                "{} ({})  [{} raw, {}]",
                trait_value.name(),
                trait_value.abbreviation(),
                signed(cost.raw),
                cost.evidence.label()
            );
            if ui
                .checkbox(&mut selected, label)
                .on_hover_text(format!(
                    "Cost evidence: {}. Runtime effect: {}.",
                    cost.evidence.label(),
                    trait_value.effect_owner_prs().join(", ")
                ))
                .changed()
            {
                if selected {
                    self.draft.lesser.insert(trait_value);
                } else {
                    self.draft.lesser.remove(&trait_value);
                }
            }
        }

        if self.draft.lesser.len() > 4 {
            ui.label(
                egui::RichText::new(
                    "More than four lesser traits triggers the recovered count penalty.",
                )
                .color(egui::Color32::from_rgb(235, 190, 110)),
            );
        }
    }

    fn show_habitability(&mut self, ui: &mut egui::Ui) {
        for (axis, name) in ["Gravity", "Temperature", "Radiation"].iter().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(*name).strong());
                    ui.checkbox(&mut self.draft.habitability[axis].immune, "Immune")
                        .on_hover_text(
                            "An immune axis has no band and contributes a fixed sample.",
                        );
                });
                ui.add_enabled_ui(!self.draft.habitability[axis].immune, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Minimum");
                        ui.add(
                            egui::DragValue::new(&mut self.draft.habitability[axis].min)
                                .range(0..=100)
                                .speed(1),
                        );
                        ui.label("Maximum");
                        ui.add(
                            egui::DragValue::new(&mut self.draft.habitability[axis].max)
                                .range(0..=100)
                                .speed(1),
                        );
                    });
                });
                let band = self.draft.habitability[axis];
                if band.immune {
                    ui.label("Ideal: immune");
                } else {
                    ui.label(format!("Ideal midpoint: {}", band.ideal()));
                }
            });
        }

        ui.add_space(8.0);
        ui.add(
            egui::Slider::new(&mut self.draft.growth_rate, GROWTH_MIN..=GROWTH_MAX)
                .text("Ideal growth rate"),
        )
        .on_hover_text("Recovered range is 1% to 20% per year. Low growth pays advantage points.");
    }

    fn show_efficiency(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::Slider::new(
                &mut self.draft.efficiency.resource_generation,
                RESOURCE_GENERATION_BOUNDS.min..=RESOURCE_GENERATION_BOUNDS.max,
            )
            .text("Colonists per resource"),
        )
        .on_hover_text("Recovered rsResGen range.");
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
        ui.checkbox(&mut self.draft.cheap_factories, "Cheap factory materials")
            .on_hover_text("Recovered CheapFact flag; costs 175 raw third-points.");

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
        ui.label(
            "Each field independently costs 50% less, standard, or 75% extra. The net count \
             drives the recovered research-point curve.",
        );
        for (index, field) in stars_core::race::spec::RESEARCH_FIELD_NAMES
            .iter()
            .enumerate()
        {
            ui.group(|ui| {
                ui.label(egui::RichText::new(*field).strong());
                ui.horizontal_wrapped(|ui| {
                    for choice in [
                        ResearchCost::Cheaper,
                        ResearchCost::Standard,
                        ResearchCost::Extra,
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
        )
        .on_hover_text("Recovered Tech 3 flag; costs 180 raw third-points.");
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
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Term");
                ui.label("Rule");
                ui.label("Raw");
                ui.end_row();
                let terms = self
                    .evaluation()
                    .map_or_else(Vec::new, |evaluation| evaluation.terms.clone());
                for term in terms.iter().filter(|term| term.raw != 0) {
                    ui.label(point_term_label(term));
                    ui.label(term.rule_id);
                    ui.label(signed_i64(term.raw));
                    ui.end_row();
                }
                let (raw, displayed) = self
                    .evaluation()
                    .map_or((0, 0), |evaluation| (evaluation.raw, evaluation.displayed));
                ui.label(egui::RichText::new("Total").strong());
                ui.label("third-points / 3");
                ui.label(
                    egui::RichText::new(format!("{} raw = {} AP", signed_i64(raw), displayed))
                        .strong(),
                );
                ui.end_row();
            });

        if self.draft.lesser.len() > 4 {
            ui.label(
                egui::RichText::new(
                    "More than four lesser traits: the count penalty is included above.",
                )
                .color(egui::Color32::from_rgb(235, 190, 110)),
            );
        }

        if let Some(primary) = self.draft.primary {
            ui.label(
                egui::RichText::new(format!(
                    "{} runtime effect is owned by {} and is not implemented in PR5.",
                    primary.name(),
                    primary.effect_owner_prs().join(", ")
                ))
                .weak(),
            );
        }
        for trait_value in &self.draft.lesser {
            ui.label(
                egui::RichText::new(format!(
                    "{} runtime effect is owned by {} and is not implemented in PR5.",
                    trait_value.name(),
                    trait_value.effect_owner_prs().join(", ")
                ))
                .weak(),
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
            ui.label(format!("{}: {error}", error.field_key()))
                .on_hover_text(match error.field() {
                    RaceField::AdvantagePoints => {
                        "Reduce advantages or add disadvantages until the balance is zero or greater."
                    }
                    RaceField::Primary => "Exactly one primary trait is required.",
                    _ => "Correct this field and finish again.",
                });
        }
    }
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

fn signed(value: i32) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
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
    use super::{RaceBuilderState, RacePage};
    use stars_core::race::{LesserTrait, PrimaryTrait, ReferenceHabitability, validate_race};

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
