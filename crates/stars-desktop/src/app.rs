//! Resizable desktop shell for the Stars! remake foundation.
//!
//! The shell contains no gameplay. It demonstrates the panel layout, scalable
//! text, procedural placeholder rendering, keyboard access, and the worker
//! request/response discipline that later PRs will reuse for turn processing.

use std::sync::mpsc::Receiver;
use std::time::Duration;

use eframe::egui;

use crate::preview::{
    DEFAULT_PREVIEW_COLUMNS, DEFAULT_PREVIEW_ROWS, PREVIEW_COORDINATE_SCALE, PreviewSnapshot,
    PreviewStar,
};
use crate::shell::ShellModel;
use crate::ui::RaceBuilderState;
use crate::worker::{RequestId, SyntheticJob, WorkerError, WorkerHandle, WorkerMessage};

/// Window title used by the native application.
pub const APP_TITLE: &str = "Stars! Remake - Foundation Shell";

const STATUS_REPAINT_INTERVAL: Duration = Duration::from_millis(50);
const SYNTHETIC_STEP_DELAY: Duration = Duration::from_millis(28);

/// State of the desktop application.
pub struct StarsApp {
    shell: ShellModel,
    worker: WorkerHandle,
    messages: Receiver<WorkerMessage>,
    next_request: u64,
    ui_scale: f32,
    fail_next_run: bool,
    show_about: bool,
    race_builder: RaceBuilderState,
    show_race_builder: bool,
}

impl StarsApp {
    /// Creates the shell and starts its background worker.
    ///
    /// # Errors
    ///
    /// Returns a typed worker error if the operating system cannot start the
    /// background thread.
    pub fn new() -> Result<Self, WorkerError> {
        let (worker, messages) = WorkerHandle::spawn()?;

        Ok(Self {
            shell: ShellModel::new(),
            worker,
            messages,
            next_request: 1,
            ui_scale: 1.0,
            fail_next_run: false,
            show_about: false,
            race_builder: RaceBuilderState::new(),
            show_race_builder: false,
        })
    }

    /// Access the headless shell model. Intended for diagnostics and tests.
    #[must_use]
    pub const fn shell(&self) -> &ShellModel {
        &self.shell
    }

    /// Access the headless custom-race builder. Intended for integration and
    /// tests; the new-game form in PR6 consumes a validated race.
    #[must_use]
    pub const fn race_builder(&self) -> &RaceBuilderState {
        &self.race_builder
    }

    /// Opens the custom-race builder, if it is not already open.
    pub fn open_race_builder(&mut self) {
        self.show_race_builder = true;
    }

    /// Access the current UI scale factor.
    #[must_use]
    pub const fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    /// Sets the UI scale, clamped to the range offered by the inspector.
    pub fn set_ui_scale(&mut self, scale: f32) {
        self.ui_scale = scale.clamp(0.75, 2.5);
    }

    /// Draws one frame of the shell.
    ///
    /// This is separated from the [`eframe::App`] implementation so headless
    /// tests can run the same layout through an `egui::Context` without a
    /// window or graphics device.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        self.drain_worker_messages();
        self.handle_shortcuts(ui.ctx());
        self.apply_ui_scale(ui.ctx());

        self.show_menu(ui);
        self.show_status(ui);
        self.show_inspector(ui);
        self.show_map(ui);
        self.show_race_builder_window(ui.ctx());
        self.show_about(ui.ctx());

        if self.shell.activity().is_running() {
            ui.ctx().request_repaint_after(STATUS_REPAINT_INTERVAL);
        }
    }

    fn start_synthetic_scan(&mut self) {
        let request_id = RequestId::new(self.next_request);
        self.next_request = self.next_request.saturating_add(1);

        let mut job = SyntheticJob::new(
            request_id,
            request_id.value() ^ 0x5eed_0000,
            DEFAULT_PREVIEW_ROWS,
            DEFAULT_PREVIEW_COLUMNS,
            SYNTHETIC_STEP_DELAY,
        );
        if self.fail_next_run {
            self.fail_next_run = false;
            job = job.failing_at(DEFAULT_PREVIEW_ROWS / 3);
        }

        if !self.shell.begin_request(request_id, DEFAULT_PREVIEW_ROWS) {
            log::warn!("refused to start non-monotonic request {request_id}");
            return;
        }

        if let Err(error) = self.worker.submit(job) {
            self.shell
                .apply(WorkerMessage::Failed { request_id, error });
        }
    }

    fn drain_worker_messages(&mut self) {
        while let Ok(message) = self.messages.try_recv() {
            let request_id = message.request_id();
            let outcome = self.shell.apply(message);
            log::debug!("worker message for {request_id}: {outcome:?}");
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (start_scan, quit) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::F5),
                input.modifiers.command && input.key_pressed(egui::Key::Q),
            )
        });

        if start_scan {
            self.start_synthetic_scan();
        }
        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn apply_ui_scale(&self, ctx: &egui::Context) {
        if (ctx.zoom_factor() - self.ui_scale).abs() > f32::EPSILON {
            ctx.set_zoom_factor(self.ui_scale);
        }
    }

    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(egui::Button::new("Custom Race Builder..."))
                        .clicked()
                    {
                        self.show_race_builder = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .add(egui::Button::new("Exit").shortcut_text("Ctrl+Q"))
                        .clicked()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close();
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.label("UI scale");
                    for (label, value) in [
                        ("100%", 1.0_f32),
                        ("125%", 1.25),
                        ("150%", 1.5),
                        ("200%", 2.0),
                    ] {
                        let selected = (self.ui_scale - value).abs() < 0.001;
                        if ui.selectable_label(selected, label).clicked() {
                            self.set_ui_scale(value);
                            ui.ctx().set_zoom_factor(self.ui_scale);
                            ui.close();
                        }
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About the foundation shell").clicked() {
                        self.show_about = true;
                        ui.close();
                    }
                });
            });
        });
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status_area").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if self.shell.activity().is_running() {
                    ui.spinner();
                } else {
                    ui.label("•");
                }
                ui.label(self.shell.status_text());

                if let Some(fraction) = self.shell.progress_fraction() {
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(180.0)
                            .show_percentage(),
                    );
                }
            });
        });
    }

    fn show_inspector(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(290.0)
            .min_size(230.0)
            .max_size(460.0)
            .show(ui, |ui| {
                ui.heading("Inspector");
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Foundation shell")
                        .strong()
                        .size(16.0),
                );
                ui.label("No game state is loaded. The map and worker use synthetic preview data.");
                ui.separator();

                ui.label(format!(
                    "Active request: {}",
                    display_request(self.shell.active_request())
                ));
                ui.label(format!(
                    "Accepted request: {}",
                    display_request(self.shell.last_accepted_request())
                ));
                ui.label(format!(
                    "Preview stars: {}",
                    self.shell
                        .snapshot()
                        .map_or(0, PreviewSnapshot::star_count)
                ));

                ui.add_space(8.0);
                ui.separator();
                ui.label("Custom race");
                match self.race_builder.validated_race() {
                    Some(race) => {
                        ui.label(format!(
                            "{} ({})",
                            race.name,
                            race.primary.abbreviation()
                        ));
                        ui.label("Validated and ready for the new-game form.");
                    }
                    None => {
                        ui.label("No validated race yet.");
                    }
                }
                if ui
                    .add(egui::Button::new("Open custom race builder").min_size(egui::vec2(0.0, 28.0)))
                    .clicked()
                {
                    self.show_race_builder = true;
                }

                ui.add_space(8.0);
                if ui
                    .add(egui::Button::new("Run synthetic scan").min_size(egui::vec2(0.0, 28.0)))
                    .on_hover_text("F5")
                    .clicked()
                {
                    self.start_synthetic_scan();
                }

                if ui
                    .checkbox(&mut self.fail_next_run, "Fail the next synthetic run")
                    .on_hover_text(
                        "Demonstrates that a worker failure retains the last accepted snapshot.",
                    )
                    .changed()
                {
                    log::debug!("fail_next_run set to {}", self.fail_next_run);
                }

                ui.add_space(8.0);
                ui.separator();
                ui.label("Display");
                ui.add(
                    egui::Slider::new(&mut self.ui_scale, 0.75..=2.5)
                        .step_by(0.05)
                        .text("UI scale"),
                );
                ui.label(format!("Current scale: {:.0}%", self.ui_scale * 100.0));
                ui.label(
                    "Layout has no fixed aspect ratio. Drag the inspector edge or resize the window.",
                );

                ui.add_space(8.0);
                ui.separator();
                ui.label("Keyboard");
                ui.label("Tab moves focus. F5 runs a scan. Ctrl+Q exits.");
            });
    }

    fn show_map(&self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            let desired_size = ui.available_size();
            let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
            let rect = response.rect;

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(7, 11, 22));

            let grid_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(22, 32, 52));
            for step in 1..10 {
                let fraction = step as f32 / 10.0;
                let x = egui::lerp(rect.left()..=rect.right(), fraction);
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    grid_stroke,
                );
            }
            for step in 1..6 {
                let fraction = step as f32 / 6.0;
                let y = egui::lerp(rect.top()..=rect.bottom(), fraction);
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    grid_stroke,
                );
            }

            if let Some(snapshot) = self.shell.snapshot() {
                paint_preview(&painter, rect, snapshot);
                painter.text(
                    rect.left_top() + egui::vec2(12.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "Map area - synthetic preview | {} stars | seed {}",
                        snapshot.star_count(),
                        snapshot.seed
                    ),
                    egui::FontId::proportional(14.0),
                    egui::Color32::from_rgb(170, 196, 224),
                );
            } else {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Map area\nRun a synthetic scan to draw a procedural preview.",
                    egui::FontId::proportional(18.0),
                    egui::Color32::from_rgb(142, 162, 188),
                );
            }

            if self.shell.activity().is_running() {
                painter.text(
                    rect.center_bottom() - egui::vec2(0.0, 18.0),
                    egui::Align2::CENTER_BOTTOM,
                    "Generating preview on the background worker...",
                    egui::FontId::proportional(13.0),
                    egui::Color32::from_rgb(140, 190, 230),
                );
            }

            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 58, 82)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                rect.right_bottom() - egui::vec2(10.0, 8.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("UI scale {:.0}%", self.ui_scale * 100.0),
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgb(105, 124, 148),
            );
        });
    }

    fn show_race_builder_window(&mut self, ctx: &egui::Context) {
        if !self.show_race_builder {
            return;
        }

        let mut open = self.show_race_builder;
        egui::Window::new("Custom Race Builder")
            .open(&mut open)
            .resizable(true)
            .default_width(820.0)
            .default_height(760.0)
            .min_width(560.0)
            .min_height(480.0)
            .show(ctx, |ui| {
                self.race_builder.draw(ui);
            });
        self.show_race_builder = open;
    }

    fn show_about(&mut self, ctx: &egui::Context) {
        egui::Window::new("About the foundation shell")
            .open(&mut self.show_about)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Stars! remake");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                ui.label(
                    "PR3 establishes the Rust workspace and desktop shell. It contains no game \
                     rules, economy, AI policy or save format.",
                );
                ui.label(format!(
                    "Core contract version: {}",
                    stars_core::CORE_API_VERSION
                ));
                ui.label("Rendering: egui/eframe with the wgpu backend.");
            });
    }
}

fn display_request(request: Option<RequestId>) -> String {
    request.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn paint_preview(painter: &egui::Painter, rect: egui::Rect, snapshot: &PreviewSnapshot) {
    let scale = f32::from(PREVIEW_COORDINATE_SCALE);
    for star in &snapshot.stars {
        let position = egui::pos2(
            rect.left() + rect.width() * (f32::from(star.x) / scale),
            rect.top() + rect.height() * (f32::from(star.y) / scale),
        );
        paint_star(painter, position, *star);
    }
}

fn paint_star(painter: &egui::Painter, position: egui::Pos2, star: PreviewStar) {
    let radius = 1.3 + f32::from(star.size) * 0.8;
    let color = egui::Color32::from_rgb(
        150 + star.intensity / 6,
        180 + star.intensity / 8,
        220 + star.intensity / 12,
    );

    painter.circle_filled(
        position,
        radius + 2.0,
        egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 28),
    );
    painter.circle_filled(position, radius, color);
}

impl eframe::App for StarsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
