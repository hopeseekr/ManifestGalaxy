//! Scalable star-map projection, interaction and painting.
//!
//! The map stores only presentation state. Planet and fleet facts come from a
//! core [`PlayerView`]; authoritative integer positions are projected into
//! logical screen coordinates here and are never written back to the game.

use eframe::egui;

use stars_core::model::{FleetId, GalacticPosition, PlanetId, PlayerId};
use stars_core::view::{FleetView, PlanetView, PlayerView, ViewLocation};

/// Lowest supported map zoom in screen points per authoritative world unit.
pub const MIN_ZOOM: f32 = 0.01;

/// Highest supported map zoom in screen points per authoritative world unit.
pub const MAX_ZOOM: f32 = 60.0;

/// Padding used by fit-to-view, in logical points.
pub const FIT_PADDING: f32 = 34.0;

/// Screen-space radius used for planet hit testing.
pub const PLANET_HIT_RADIUS: f32 = 13.0;

/// Screen-space radius used for fleet hit testing.
pub const FLEET_HIT_RADIUS: f32 = 11.0;

/// One selected entity, identified by its stable game identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Selection {
    /// No selection.
    #[default]
    None,
    /// A planet.
    Planet(PlanetId),
    /// A fleet.
    Fleet(FleetId),
}

impl Selection {
    /// Returns whether the selected entity still exists in the view.
    ///
    /// A turn can remove or hide an entity. Selection is retained by identifier
    /// while it remains visible and cleared as soon as it disappears.
    #[must_use]
    pub fn retain(self, view: &PlayerView) -> Self {
        match self {
            Self::Planet(id) if view.planets.iter().any(|planet| planet.id == id) => self,
            Self::Fleet(id) if view.fleets.iter().any(|fleet| fleet.id == id) => self,
            Self::None => self,
            Self::Planet(_) | Self::Fleet(_) => Self::None,
        }
    }

    /// Selected planet identifier, if any.
    #[must_use]
    pub const fn planet(self) -> Option<PlanetId> {
        match self {
            Self::Planet(id) => Some(id),
            Self::Fleet(_) | Self::None => None,
        }
    }

    /// Selected fleet identifier, if any.
    #[must_use]
    pub const fn fleet(self) -> Option<FleetId> {
        match self {
            Self::Fleet(id) => Some(id),
            Self::Planet(_) | Self::None => None,
        }
    }
}

/// Integer world bounds of visible map objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBounds {
    /// Minimum horizontal world coordinate.
    pub min_x: f32,
    /// Minimum vertical world coordinate.
    pub min_y: f32,
    /// Maximum horizontal world coordinate.
    pub max_x: f32,
    /// Maximum vertical world coordinate.
    pub max_y: f32,
}

impl WorldBounds {
    /// Finds the bounds of all visible planets and positioned fleets.
    ///
    /// Fleets at unknown orbits are deliberately skipped: the player view does
    /// not reveal where they are.
    #[must_use]
    pub fn from_view(view: &PlayerView) -> Option<Self> {
        let mut bounds: Option<Self> = None;
        for planet in &view.planets {
            bounds = Some(bounds.map_or(Self::single(planet.position), |current| {
                current.include(planet.position)
            }));
        }
        for fleet in &view.fleets {
            if let ViewLocation::InSpace(position) = fleet.location {
                bounds = Some(
                    bounds.map_or(Self::single(position), |current| current.include(position)),
                );
            }
        }
        bounds
    }

    fn single(position: GalacticPosition) -> Self {
        Self {
            min_x: coordinate(position.x),
            min_y: coordinate(position.y),
            max_x: coordinate(position.x),
            max_y: coordinate(position.y),
        }
    }

    fn include(self, position: GalacticPosition) -> Self {
        Self {
            min_x: self.min_x.min(coordinate(position.x)),
            min_y: self.min_y.min(coordinate(position.y)),
            max_x: self.max_x.max(coordinate(position.x)),
            max_y: self.max_y.max(coordinate(position.y)),
        }
    }

    /// Width in world units, at least one.
    #[must_use]
    pub fn width(self) -> f32 {
        (self.max_x - self.min_x).max(1.0)
    }

    /// Height in world units, at least one.
    #[must_use]
    pub fn height(self) -> f32 {
        (self.max_y - self.min_y).max(1.0)
    }
}

/// Cursor-centred zoom and pan projection between world and screen space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapTransform {
    viewport: egui::Rect,
    center: egui::Pos2,
    zoom: f32,
}

impl MapTransform {
    /// Creates an identity projection for a viewport.
    #[must_use]
    pub fn new(viewport: egui::Rect) -> Self {
        Self {
            viewport,
            center: egui::Pos2::ZERO,
            zoom: 1.0,
        }
    }

    /// Fits world bounds into a viewport, preserving aspect ratio.
    #[must_use]
    pub fn fit(bounds: WorldBounds, viewport: egui::Rect) -> Self {
        let usable = egui::vec2(
            (viewport.width() - FIT_PADDING * 2.0).max(1.0),
            (viewport.height() - FIT_PADDING * 2.0).max(1.0),
        );
        let zoom = (usable.x / bounds.width())
            .min(usable.y / bounds.height())
            .clamp(MIN_ZOOM, MAX_ZOOM);
        Self {
            viewport,
            center: egui::pos2(
                (bounds.min_x + bounds.max_x) * 0.5,
                (bounds.min_y + bounds.max_y) * 0.5,
            ),
            zoom,
        }
    }

    /// Current logical viewport.
    #[must_use]
    pub const fn viewport(self) -> egui::Rect {
        self.viewport
    }

    /// Current zoom in screen points per world unit.
    #[must_use]
    pub const fn zoom(self) -> f32 {
        self.zoom
    }

    /// Current world-space centre.
    #[must_use]
    pub const fn center(self) -> egui::Pos2 {
        self.center
    }

    /// Updates the logical viewport without changing the world centre or zoom.
    pub fn set_viewport(&mut self, viewport: egui::Rect) {
        self.viewport = viewport;
    }

    /// Fits the supplied bounds and updates the viewport.
    pub fn fit_in_viewport(&mut self, bounds: WorldBounds) {
        *self = Self::fit(bounds, self.viewport);
    }

    /// Projects an authoritative integer position to a logical screen point.
    #[must_use]
    pub fn world_to_screen(self, position: GalacticPosition) -> egui::Pos2 {
        self.viewport.center()
            + egui::vec2(
                (coordinate(position.x) - self.center.x) * self.zoom,
                (coordinate(position.y) - self.center.y) * self.zoom,
            )
    }

    /// Converts a logical screen point to floating-point world coordinates.
    ///
    /// The result is presentation-only. Round only when an order actually needs
    /// an authoritative integer coordinate.
    #[must_use]
    pub fn screen_to_world(self, position: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            self.center.x + (position.x - self.viewport.center().x) / self.zoom,
            self.center.y + (position.y - self.viewport.center().y) / self.zoom,
        )
    }

    /// Zooms around the cursor while keeping that world point under it.
    ///
    /// Zero, negative and non-finite factors are ignored so a malformed input
    /// event cannot collapse or invert the projection.
    pub fn zoom_at(&mut self, cursor: egui::Pos2, factor: f32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let world_before = self.screen_to_world(cursor);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let world_after = self.screen_to_world(cursor);
        self.center -= world_after - world_before;
    }

    /// Pans the view by a logical screen delta.
    ///
    /// Content follows the drag: dragging right moves the world centre left.
    pub fn pan_by(&mut self, delta: egui::Vec2) {
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return;
        }
        self.center -= delta / self.zoom;
    }

    /// Returns whether a world point is inside the viewport plus a margin.
    #[must_use]
    pub fn is_visible(self, position: GalacticPosition, margin: f32) -> bool {
        let point = self.world_to_screen(position);
        self.viewport
            .expand(margin)
            .contains(egui::pos2(point.x, point.y))
    }

    /// Returns the approximate world-space bounds currently on screen.
    #[must_use]
    pub fn visible_world_bounds(self) -> WorldBounds {
        let top_left = self.screen_to_world(self.viewport.left_top());
        let bottom_right = self.screen_to_world(self.viewport.right_bottom());
        WorldBounds {
            min_x: top_left.x.min(bottom_right.x),
            min_y: top_left.y.min(bottom_right.y),
            max_x: top_left.x.max(bottom_right.x),
            max_y: top_left.y.max(bottom_right.y),
        }
    }
}

/// Headless map state used by the window and acceptance tests.
#[derive(Clone, Debug)]
pub struct MapState {
    transform: MapTransform,
    selection: Selection,
    hovered: Selection,
    needs_fit: bool,
    fitted_view: Option<(u64, u32, u32)>,
}

impl Default for MapState {
    fn default() -> Self {
        Self::new()
    }
}

impl MapState {
    /// Creates an unfitted map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            transform: MapTransform::new(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1.0, 1.0),
            )),
            selection: Selection::None,
            hovered: Selection::None,
            needs_fit: true,
            fitted_view: None,
        }
    }

    /// Current projection.
    #[must_use]
    pub const fn transform(&self) -> &MapTransform {
        &self.transform
    }

    /// Mutable projection. Intended for interaction and tests.
    pub fn transform_mut(&mut self) -> &mut MapTransform {
        &mut self.transform
    }

    /// Current linked selection.
    #[must_use]
    pub const fn selection(&self) -> Selection {
        self.selection
    }

    /// Mutable linked selection shared by reports and the inspector.
    pub fn selection_mut(&mut self) -> &mut Selection {
        &mut self.selection
    }

    /// Replaces the selection.
    pub fn select(&mut self, selection: Selection) {
        self.selection = selection;
    }

    /// Clears selection and hover state.
    pub fn clear_interaction(&mut self) {
        self.selection = Selection::None;
        self.hovered = Selection::None;
    }

    /// Requests a fit on the next frame.
    pub fn request_fit(&mut self) {
        self.needs_fit = true;
    }

    /// Fits the view when it was requested, the viewport changed materially, or
    /// a different universe is shown.
    pub fn prepare(&mut self, view: &PlayerView, viewport: egui::Rect) {
        let key = (
            view.planets.first().map_or(0, |planet| planet.id.raw()),
            view.turn,
            view.year,
        );
        let viewport_changed = (self.transform.viewport().size() - viewport.size()).length() > 0.5;
        self.transform.set_viewport(viewport);
        self.selection = self.selection.retain(view);
        if self.needs_fit || self.fitted_view != Some(key) || viewport_changed {
            if let Some(bounds) = WorldBounds::from_view(view) {
                self.transform.fit_in_viewport(bounds);
            }
            self.needs_fit = false;
            self.fitted_view = Some(key);
        }
    }

    /// Fits all visible objects into the current viewport.
    pub fn focus(&mut self, view: &PlayerView) {
        if let Some(bounds) = WorldBounds::from_view(view) {
            self.transform.fit_in_viewport(bounds);
        }
    }

    /// Centres the current selection without changing zoom.
    pub fn focus_selection(&mut self, view: &PlayerView) {
        if let Some(position) = self.selected_world_position(view) {
            self.transform.center = egui::pos2(coordinate(position.x), coordinate(position.y));
        }
    }

    /// Selected world position, if the selected entity is visible.
    #[must_use]
    pub fn selected_world_position(&self, view: &PlayerView) -> Option<GalacticPosition> {
        match self.selection {
            Selection::Planet(id) => view
                .planets
                .iter()
                .find(|planet| planet.id == id)
                .map(|planet| planet.position),
            Selection::Fleet(id) => view
                .fleets
                .iter()
                .find(|fleet| fleet.id == id)
                .and_then(visible_fleet_position),
            Selection::None => None,
        }
    }

    /// Finds the closest selectable entity under a logical screen point.
    ///
    /// Fleets at a planet are drawn with a small screen-space offset, so their
    /// hit target remains distinguishable from the planet beneath them.
    #[must_use]
    pub fn hit_test(&self, view: &PlayerView, pointer: egui::Pos2) -> Selection {
        let mut best = Selection::None;
        let mut best_distance = f32::INFINITY;

        for fleet in &view.fleets {
            let Some(position) = fleet_screen_position(&self.transform, fleet) else {
                continue;
            };
            let distance = position.distance(pointer);
            if distance <= FLEET_HIT_RADIUS && distance < best_distance {
                best = Selection::Fleet(fleet.id);
                best_distance = distance;
            }
        }

        for planet in &view.planets {
            let distance = self
                .transform
                .world_to_screen(planet.position)
                .distance(pointer);
            if distance <= PLANET_HIT_RADIUS && distance < best_distance {
                best = Selection::Planet(planet.id);
                best_distance = distance;
            }
        }

        best
    }

    /// Planets currently inside the viewport plus a margin.
    pub fn visible_planets<'a>(
        &self,
        view: &'a PlayerView,
    ) -> impl Iterator<Item = &'a PlanetView> + 'a {
        let transform = self.transform;
        view.planets
            .iter()
            .filter(move |planet| transform.is_visible(planet.position, 24.0))
    }

    /// Fleets currently inside the viewport plus a margin.
    pub fn visible_fleets<'a>(
        &self,
        view: &'a PlayerView,
    ) -> impl Iterator<Item = &'a FleetView> + 'a {
        let transform = self.transform;
        view.fleets.iter().filter(move |fleet| {
            visible_fleet_position(fleet)
                .is_some_and(|position| transform.is_visible(position, 24.0))
        })
    }

    /// Number of planet and fleet objects that would be painted right now.
    #[must_use]
    pub fn render_counts(&self, view: &PlayerView) -> (usize, usize) {
        (
            self.visible_planets(view).count(),
            self.visible_fleets(view).count(),
        )
    }

    /// Draws the map and applies pointer and keyboard interaction.
    pub fn draw(&mut self, ui: &mut egui::Ui, view: &PlayerView) {
        let desired = ui.available_size();
        let (response, painter) = ui.allocate_painter(
            desired,
            egui::Sense::click_and_drag().union(egui::Sense::focusable_noninteractive()),
        );
        let viewport = response.rect;
        self.prepare(view, viewport);

        if response.clicked() {
            response.request_focus();
        }
        if response.dragged() {
            self.transform.pan_by(response.drag_delta());
        }

        if let Some(pointer) = response.hover_pos() {
            let scroll = ui.input(|input| input.smooth_scroll_delta().y);
            if scroll.abs() > 0.01 {
                let factor = (scroll * 0.0025).exp();
                self.transform.zoom_at(pointer, factor);
            }
            self.hovered = self.hit_test(view, pointer);
        } else {
            self.hovered = Selection::None;
        }

        if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            self.selection = self.hit_test(view, pointer);
        }

        if response.has_focus() {
            self.handle_keyboard(ui, view);
        }

        painter.rect_filled(viewport, 0.0, egui::Color32::from_rgb(7, 11, 22));
        self.paint_grid(&painter, viewport);
        self.paint_objects(&painter, view);
        self.paint_boundary(&painter, viewport);
        self.paint_caption(&painter, viewport, view);
    }

    fn handle_keyboard(&mut self, ui: &egui::Ui, view: &PlayerView) {
        let (left, right, up, down, plus, minus, fit, escape) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::ArrowLeft),
                input.key_pressed(egui::Key::ArrowRight),
                input.key_pressed(egui::Key::ArrowUp),
                input.key_pressed(egui::Key::ArrowDown),
                input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals),
                input.key_pressed(egui::Key::Minus),
                input.key_pressed(egui::Key::F),
                input.key_pressed(egui::Key::Escape),
            )
        });

        let step = 48.0;
        if left {
            self.transform.pan_by(egui::vec2(step, 0.0));
        }
        if right {
            self.transform.pan_by(egui::vec2(-step, 0.0));
        }
        if up {
            self.transform.pan_by(egui::vec2(0.0, step));
        }
        if down {
            self.transform.pan_by(egui::vec2(0.0, -step));
        }
        if plus {
            let centre = self.transform.viewport().center();
            self.transform.zoom_at(centre, 1.2);
        }
        if minus {
            let centre = self.transform.viewport().center();
            self.transform.zoom_at(centre, 1.0 / 1.2);
        }
        if fit {
            self.focus(view);
        }
        if escape {
            self.clear_interaction();
        }
    }

    fn paint_grid(&self, painter: &egui::Painter, viewport: egui::Rect) {
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(22, 32, 52));
        for step in 1..10 {
            let fraction = step as f32 / 10.0;
            let x = egui::lerp(viewport.left()..=viewport.right(), fraction);
            painter.line_segment(
                [
                    egui::pos2(x, viewport.top()),
                    egui::pos2(x, viewport.bottom()),
                ],
                stroke,
            );
        }
        for step in 1..8 {
            let fraction = step as f32 / 8.0;
            let y = egui::lerp(viewport.top()..=viewport.bottom(), fraction);
            painter.line_segment(
                [
                    egui::pos2(viewport.left(), y),
                    egui::pos2(viewport.right(), y),
                ],
                stroke,
            );
        }
    }

    fn paint_objects(&self, painter: &egui::Painter, view: &PlayerView) {
        for planet in self.visible_planets(view) {
            let position = self.transform.world_to_screen(planet.position);
            let color = owner_color(planet.owner, view.player);
            let radius = if planet.homeworld { 6.0 } else { 4.0 };
            painter.circle_filled(
                position,
                radius + 3.0,
                egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 32),
            );
            painter.circle_filled(position, radius, color);
            if planet.starbase {
                painter.circle_stroke(
                    position,
                    radius + 2.5,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(196, 210, 228)),
                );
            }
            if self.selection == Selection::Planet(planet.id) {
                painter.circle_stroke(
                    position,
                    radius + 6.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 214, 112)),
                );
            } else if self.hovered == Selection::Planet(planet.id) {
                painter.circle_stroke(
                    position,
                    radius + 4.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 224, 240)),
                );
            }
        }

        for fleet in self.visible_fleets(view) {
            let Some(position) = fleet_screen_position(&self.transform, fleet) else {
                continue;
            };
            let color = owner_color(Some(fleet.owner), view.player);
            let selected = self.selection == Selection::Fleet(fleet.id);
            let hovered = self.hovered == Selection::Fleet(fleet.id);
            let size = if selected { 7.0 } else { 5.0 };
            painter.add(egui::Shape::convex_polygon(
                vec![
                    position + egui::vec2(0.0, -size),
                    position + egui::vec2(size, size),
                    position + egui::vec2(0.0, size * 0.35),
                    position + egui::vec2(-size, size),
                ],
                color,
                if selected {
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 214, 112))
                } else if hovered {
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 232, 244))
                } else {
                    egui::Stroke::NONE
                },
            ));
        }

        if let Selection::Planet(id) = self.selection
            && let Some(planet) = view.planets.iter().find(|planet| planet.id == id)
        {
            let position = self.transform.world_to_screen(planet.position);
            painter.text(
                position + egui::vec2(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                &planet.name,
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(220, 230, 242),
            );
        }
        if let Selection::Fleet(id) = self.selection
            && let Some(fleet) = view.fleets.iter().find(|fleet| fleet.id == id)
            && let Some(position) = fleet_screen_position(&self.transform, fleet)
        {
            painter.text(
                position + egui::vec2(8.0, 10.0),
                egui::Align2::LEFT_TOP,
                &fleet.name,
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(220, 230, 242),
            );
        }
    }

    fn paint_boundary(&self, painter: &egui::Painter, viewport: egui::Rect) {
        painter.rect_stroke(
            viewport,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 58, 82)),
            egui::StrokeKind::Inside,
        );
    }

    fn paint_caption(&self, painter: &egui::Painter, viewport: egui::Rect, view: &PlayerView) {
        painter.text(
            viewport.left_top() + egui::vec2(12.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!(
                "{} - {} | turn {} | year {} | {} planets | {:.0}% zoom",
                view.player_name,
                view.role_label(),
                view.turn,
                view.year,
                view.planets.len(),
                self.transform.zoom() * 100.0
            ),
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(170, 196, 224),
        );
        painter.text(
            viewport.left_bottom() + egui::vec2(12.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag to pan | wheel to zoom at cursor | click to select | F fits | arrows pan | Esc clears",
            egui::FontId::monospace(10.0),
            egui::Color32::from_rgb(112, 132, 158),
        );
    }
}

fn coordinate(value: i32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    {
        value as f32
    }
}

fn visible_fleet_position(fleet: &FleetView) -> Option<GalacticPosition> {
    match fleet.location {
        ViewLocation::AtPlanet { position, .. } | ViewLocation::InSpace(position) => Some(position),
        ViewLocation::UnknownOrbit => None,
    }
}

fn fleet_screen_position(transform: &MapTransform, fleet: &FleetView) -> Option<egui::Pos2> {
    let position = visible_fleet_position(fleet)?;
    let mut screen = transform.world_to_screen(position);
    if matches!(fleet.location, ViewLocation::AtPlanet { .. }) {
        screen += egui::vec2(9.0, -9.0);
    }
    Some(screen)
}

fn owner_color(owner: Option<PlayerId>, viewer: PlayerId) -> egui::Color32 {
    let Some(owner) = owner else {
        return egui::Color32::from_rgb(144, 154, 170);
    };
    if owner == viewer {
        return egui::Color32::from_rgb(104, 206, 236);
    }
    const PALETTE: [egui::Color32; 8] = [
        egui::Color32::from_rgb(232, 134, 112),
        egui::Color32::from_rgb(222, 188, 104),
        egui::Color32::from_rgb(178, 146, 232),
        egui::Color32::from_rgb(120, 210, 150),
        egui::Color32::from_rgb(222, 130, 184),
        egui::Color32::from_rgb(144, 184, 232),
        egui::Color32::from_rgb(216, 160, 112),
        egui::Color32::from_rgb(168, 214, 200),
    ];
    let index = usize::try_from(owner.raw() % 8).unwrap_or(0);
    PALETTE[index]
}

trait RoleLabel {
    fn role_label(&self) -> &'static str;
}

impl RoleLabel for PlayerView {
    fn role_label(&self) -> &'static str {
        match self.role {
            stars_core::model::PlayerRole::Human => "human",
            stars_core::model::PlayerRole::Computer => "computer",
        }
    }
}
