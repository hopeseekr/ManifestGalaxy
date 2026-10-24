//! Sortable, searchable planet and fleet reports over a [`PlayerView`].
//!
//! The report model contains no game rules or authoritative state. It filters
//! and sorts the exact entity list supplied by core, preserves selection by
//! stable identifier and drops selections whose entity disappeared from the
//! view.

use eframe::egui;

use stars_core::model::{FleetId, PlanetId, PlayerId};
use stars_core::view::{FleetView, PlanetView, PlayerView, ViewLocation};

use super::map::Selection;

/// Which report table is visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReportTable {
    /// Planet report.
    #[default]
    Planets,
    /// Fleet report.
    Fleets,
}

/// Ownership filter applied to both tables.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OwnerFilter {
    /// Every visible entity.
    #[default]
    All,
    /// Entities owned by the viewing player.
    Mine,
    /// Entities owned by another player.
    Opponents,
    /// Planets with no owner.
    Unowned,
}

impl OwnerFilter {
    /// Every filter in menu order.
    pub const ALL: [Self; 4] = [Self::All, Self::Mine, Self::Opponents, Self::Unowned];

    /// Menu label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All owners",
            Self::Mine => "Mine",
            Self::Opponents => "Opponents",
            Self::Unowned => "Unowned",
        }
    }

    fn accepts(self, owner: Option<PlayerId>, viewer: PlayerId) -> bool {
        match self {
            Self::All => true,
            Self::Mine => owner == Some(viewer),
            Self::Opponents => owner.is_some_and(|owner| owner != viewer),
            Self::Unowned => owner.is_none(),
        }
    }
}

/// Planet sort columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlanetSortKey {
    /// Display name.
    #[default]
    Name,
    /// Owner label.
    Owner,
    /// Colonist population.
    Population,
    /// Gravity value.
    Gravity,
    /// Temperature value.
    Temperature,
    /// Radiation value.
    Radiation,
    /// Ironium concentration.
    Ironium,
    /// Boranium concentration.
    Boranium,
    /// Germanium concentration.
    Germanium,
    /// Factories.
    Factories,
    /// Mines.
    Mines,
    /// Defenses.
    Defenses,
}

impl PlanetSortKey {
    /// Header label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "Planet",
            Self::Owner => "Owner",
            Self::Population => "Population",
            Self::Gravity => "Gravity",
            Self::Temperature => "Temperature",
            Self::Radiation => "Radiation",
            Self::Ironium => "Ironium",
            Self::Boranium => "Boranium",
            Self::Germanium => "Germanium",
            Self::Factories => "Factories",
            Self::Mines => "Mines",
            Self::Defenses => "Defenses",
        }
    }
}

/// Fleet sort columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FleetSortKey {
    /// Display name.
    #[default]
    Name,
    /// Owner label.
    Owner,
    /// Ship count.
    Ships,
    /// Visible design name.
    Design,
    /// Location description.
    Location,
}

impl FleetSortKey {
    /// Header label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "Fleet",
            Self::Owner => "Owner",
            Self::Ships => "Ships",
            Self::Design => "Design",
            Self::Location => "Location",
        }
    }
}

/// Headless report filter and sort state.
#[derive(Clone, Debug)]
pub struct ReportState {
    table: ReportTable,
    query: String,
    owner_filter: OwnerFilter,
    planet_sort: PlanetSortKey,
    planet_ascending: bool,
    fleet_sort: FleetSortKey,
    fleet_ascending: bool,
}

impl Default for ReportState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportState {
    /// Creates a name-sorted, unfiltered report.
    #[must_use]
    pub fn new() -> Self {
        Self {
            table: ReportTable::Planets,
            query: String::new(),
            owner_filter: OwnerFilter::All,
            planet_sort: PlanetSortKey::Name,
            planet_ascending: true,
            fleet_sort: FleetSortKey::Name,
            fleet_ascending: true,
        }
    }

    /// Active table.
    #[must_use]
    pub const fn table(&self) -> ReportTable {
        self.table
    }

    /// Chooses the active table.
    pub fn set_table(&mut self, table: ReportTable) {
        self.table = table;
    }

    /// Current search text.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Replaces the search text.
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
    }

    /// Mutable search field for the UI.
    pub fn query_mut(&mut self) -> &mut String {
        &mut self.query
    }

    /// Current ownership filter.
    #[must_use]
    pub const fn owner_filter(&self) -> OwnerFilter {
        self.owner_filter
    }

    /// Replaces the ownership filter.
    pub fn set_owner_filter(&mut self, filter: OwnerFilter) {
        self.owner_filter = filter;
    }

    /// Current planet sort column.
    #[must_use]
    pub const fn planet_sort(&self) -> PlanetSortKey {
        self.planet_sort
    }

    /// Whether the planet table sorts ascending.
    #[must_use]
    pub const fn planet_ascending(&self) -> bool {
        self.planet_ascending
    }

    /// Sorts by a planet column, toggling direction when it is already active.
    pub fn sort_planets_by(&mut self, key: PlanetSortKey) {
        if self.planet_sort == key {
            self.planet_ascending = !self.planet_ascending;
        } else {
            self.planet_sort = key;
            self.planet_ascending = true;
        }
    }

    /// Current fleet sort column.
    #[must_use]
    pub const fn fleet_sort(&self) -> FleetSortKey {
        self.fleet_sort
    }

    /// Whether the fleet table sorts ascending.
    #[must_use]
    pub const fn fleet_ascending(&self) -> bool {
        self.fleet_ascending
    }

    /// Sorts by a fleet column, toggling direction when it is already active.
    pub fn sort_fleets_by(&mut self, key: FleetSortKey) {
        if self.fleet_sort == key {
            self.fleet_ascending = !self.fleet_ascending;
        } else {
            self.fleet_sort = key;
            self.fleet_ascending = true;
        }
    }

    /// Filtered and sorted planets. Ties are broken by stable identifier.
    #[must_use]
    pub fn visible_planets<'a>(&self, view: &'a PlayerView) -> Vec<&'a PlanetView> {
        let query = self.query.trim().to_lowercase();
        let mut planets: Vec<&PlanetView> = view
            .planets
            .iter()
            .filter(|planet| self.owner_filter.accepts(planet.owner, view.player))
            .filter(|planet| {
                query.is_empty()
                    || planet.name.to_lowercase().contains(&query)
                    || owner_label(view, planet.owner)
                        .to_lowercase()
                        .contains(&query)
                    || planet
                        .environment
                        .values()
                        .iter()
                        .any(|value| value.to_string().contains(&query))
            })
            .collect();

        planets.sort_by(|left, right| {
            let order = compare_planets(view, self.planet_sort, left, right);
            let order = if self.planet_ascending {
                order
            } else {
                order.reverse()
            };
            order.then_with(|| left.id.cmp(&right.id))
        });
        planets
    }

    /// Filtered and sorted fleets. Ties are broken by stable identifier.
    #[must_use]
    pub fn visible_fleets<'a>(&self, view: &'a PlayerView) -> Vec<&'a FleetView> {
        let query = self.query.trim().to_lowercase();
        let mut fleets: Vec<&FleetView> = view
            .fleets
            .iter()
            .filter(|fleet| self.owner_filter.accepts(Some(fleet.owner), view.player))
            .filter(|fleet| {
                query.is_empty()
                    || fleet.name.to_lowercase().contains(&query)
                    || owner_label(view, Some(fleet.owner))
                        .to_lowercase()
                        .contains(&query)
                    || design_label(view, fleet).to_lowercase().contains(&query)
                    || location_label(view, fleet).to_lowercase().contains(&query)
            })
            .collect();

        fleets.sort_by(|left, right| {
            let order = compare_fleets(view, self.fleet_sort, left, right);
            let order = if self.fleet_ascending {
                order
            } else {
                order.reverse()
            };
            order.then_with(|| left.id.cmp(&right.id))
        });
        fleets
    }

    /// Drops a selection whose entity is no longer in the view.
    pub fn retain_selection(&self, view: &PlayerView, selection: &mut Selection) {
        *selection = selection.retain(view);
    }

    /// Draws the report panel. Clicking a row updates the shared selection.
    pub fn draw(&mut self, ui: &mut egui::Ui, view: &PlayerView, selection: &mut Selection) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.table, ReportTable::Planets, "Planets");
            ui.selectable_value(&mut self.table, ReportTable::Fleets, "Fleets");
        });
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("name, owner, value")
                    .desired_width(f32::INFINITY),
            );
        });
        egui::ComboBox::from_id_salt("report_owner_filter")
            .selected_text(self.owner_filter.label())
            .show_ui(ui, |ui| {
                for filter in OwnerFilter::ALL {
                    ui.selectable_value(&mut self.owner_filter, filter, filter.label());
                }
            });
        ui.separator();

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.table {
                ReportTable::Planets => self.draw_planets(ui, view, selection),
                ReportTable::Fleets => self.draw_fleets(ui, view, selection),
            });
    }

    fn draw_planets(&mut self, ui: &mut egui::Ui, view: &PlayerView, selection: &mut Selection) {
        let planets = self.visible_planets(view);
        ui.label(format!("{} visible planets", planets.len()));
        egui::Grid::new("planet_report_grid")
            .num_columns(6)
            .striped(true)
            .spacing([12.0, 3.0])
            .show(ui, |ui| {
                for key in [
                    PlanetSortKey::Name,
                    PlanetSortKey::Owner,
                    PlanetSortKey::Population,
                    PlanetSortKey::Gravity,
                    PlanetSortKey::Temperature,
                    PlanetSortKey::Radiation,
                ] {
                    let arrow = sort_arrow(self.planet_sort == key, self.planet_ascending);
                    if ui.small_button(format!("{}{arrow}", key.label())).clicked() {
                        self.sort_planets_by(key);
                    }
                }
                ui.end_row();

                for planet in planets {
                    let selected = *selection == Selection::Planet(planet.id);
                    if ui
                        .selectable_label(selected, &planet.name)
                        .on_hover_text(format!(
                            "Gravity {} | Temperature {} | Radiation {}",
                            planet.environment.gravity,
                            planet.environment.temperature,
                            planet.environment.radiation
                        ))
                        .clicked()
                    {
                        *selection = Selection::Planet(planet.id);
                    }
                    ui.label(owner_label(view, planet.owner));
                    ui.label(format_integer(planet.population));
                    ui.label(planet.environment.gravity.to_string());
                    ui.label(planet.environment.temperature.to_string());
                    ui.label(planet.environment.radiation.to_string());
                    ui.end_row();
                }
            });
    }

    fn draw_fleets(&mut self, ui: &mut egui::Ui, view: &PlayerView, selection: &mut Selection) {
        let fleets = self.visible_fleets(view);
        ui.label(format!("{} visible fleets", fleets.len()));
        egui::Grid::new("fleet_report_grid")
            .num_columns(5)
            .striped(true)
            .spacing([12.0, 3.0])
            .show(ui, |ui| {
                for key in [
                    FleetSortKey::Name,
                    FleetSortKey::Owner,
                    FleetSortKey::Ships,
                    FleetSortKey::Design,
                    FleetSortKey::Location,
                ] {
                    let arrow = sort_arrow(self.fleet_sort == key, self.fleet_ascending);
                    if ui.small_button(format!("{}{arrow}", key.label())).clicked() {
                        self.sort_fleets_by(key);
                    }
                }
                ui.end_row();

                for fleet in fleets {
                    let selected = *selection == Selection::Fleet(fleet.id);
                    if ui.selectable_label(selected, &fleet.name).clicked() {
                        *selection = Selection::Fleet(fleet.id);
                    }
                    ui.label(owner_label(view, Some(fleet.owner)));
                    ui.label(fleet.ships.to_string());
                    ui.label(design_label(view, fleet));
                    ui.label(location_label(view, fleet));
                    ui.end_row();
                }
            });
    }
}

/// Owner text shown in reports and inspectors.
#[must_use]
pub fn owner_label(view: &PlayerView, owner: Option<PlayerId>) -> String {
    match owner {
        None => "Unowned".to_owned(),
        Some(owner) if owner == view.player => format!("{} (you)", view.player_name),
        Some(owner) => format!("Player {}", owner.raw()),
    }
}

/// Visible design name for a fleet.
#[must_use]
pub fn design_label(view: &PlayerView, fleet: &FleetView) -> String {
    fleet.design.map_or_else(
        || "Unknown design".to_owned(),
        |id| {
            view.designs
                .iter()
                .find(|design| design.id == id)
                .map_or_else(|| "Unknown design".to_owned(), |design| design.name.clone())
        },
    )
}

/// Fleet location text without leaking an invisible planet identity.
#[must_use]
pub fn location_label(view: &PlayerView, fleet: &FleetView) -> String {
    match fleet.location {
        ViewLocation::AtPlanet { planet, .. } => view
            .planets
            .iter()
            .find(|candidate| candidate.id == planet)
            .map_or_else(
                || "In orbit at an unknown planet".to_owned(),
                |planet| format!("Orbit: {}", planet.name),
            ),
        ViewLocation::UnknownOrbit => "Unknown orbit".to_owned(),
        ViewLocation::InSpace(position) => {
            format!("Deep space ({}, {})", position.x, position.y)
        }
    }
}

fn compare_planets(
    view: &PlayerView,
    key: PlanetSortKey,
    left: &PlanetView,
    right: &PlanetView,
) -> std::cmp::Ordering {
    match key {
        PlanetSortKey::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        PlanetSortKey::Owner => owner_label(view, left.owner).cmp(&owner_label(view, right.owner)),
        PlanetSortKey::Population => left.population.cmp(&right.population),
        PlanetSortKey::Gravity => left.environment.gravity.cmp(&right.environment.gravity),
        PlanetSortKey::Temperature => left
            .environment
            .temperature
            .cmp(&right.environment.temperature),
        PlanetSortKey::Radiation => left.environment.radiation.cmp(&right.environment.radiation),
        PlanetSortKey::Ironium => {
            left.mineral_concentration[0].cmp(&right.mineral_concentration[0])
        }
        PlanetSortKey::Boranium => {
            left.mineral_concentration[1].cmp(&right.mineral_concentration[1])
        }
        PlanetSortKey::Germanium => {
            left.mineral_concentration[2].cmp(&right.mineral_concentration[2])
        }
        PlanetSortKey::Factories => left.factories.cmp(&right.factories),
        PlanetSortKey::Mines => left.mines.cmp(&right.mines),
        PlanetSortKey::Defenses => left.defenses.cmp(&right.defenses),
    }
}

fn compare_fleets(
    view: &PlayerView,
    key: FleetSortKey,
    left: &FleetView,
    right: &FleetView,
) -> std::cmp::Ordering {
    match key {
        FleetSortKey::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        FleetSortKey::Owner => {
            owner_label(view, Some(left.owner)).cmp(&owner_label(view, Some(right.owner)))
        }
        FleetSortKey::Ships => left.ships.cmp(&right.ships),
        FleetSortKey::Design => design_label(view, left).cmp(&design_label(view, right)),
        FleetSortKey::Location => location_label(view, left).cmp(&location_label(view, right)),
    }
}

/// Formats an integer with thin thousands separators.
#[must_use]
pub fn format_integer(value: u32) -> String {
    let digits = value.to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push(',');
        }
        output.push(character);
    }
    output
}

fn sort_arrow(active: bool, ascending: bool) -> &'static str {
    if !active {
        ""
    } else if ascending {
        " ^"
    } else {
        " v"
    }
}

/// Returns planet by stable identifier from a view.
#[must_use]
pub fn planet(view: &PlayerView, id: PlanetId) -> Option<&PlanetView> {
    view.planets.iter().find(|planet| planet.id == id)
}

/// Returns fleet by stable identifier from a view.
#[must_use]
pub fn fleet(view: &PlayerView, id: FleetId) -> Option<&FleetView> {
    view.fleets.iter().find(|fleet| fleet.id == id)
}
