//! Linked planet, fleet and empire inspector.
//!
//! The inspector displays only facts already present in the core
//! [`PlayerView`]. It performs no habitability, production or combat
//! calculation.

use eframe::egui;

use stars_core::model::PlayerRole;
use stars_core::view::{FleetView, PlanetView, PlayerView};

use super::map::Selection;
use super::reports::{design_label, format_integer, location_label, owner_label};

/// Draws the inspector for the current linked selection.
pub fn draw(ui: &mut egui::Ui, view: &PlayerView, selection: Selection) {
    ui.heading("Inspector");
    ui.label(format!(
        "{} | {} | year {}",
        view.player_name,
        role_label(view.role),
        view.year
    ));
    ui.separator();

    match selection {
        Selection::None => {
            ui.label("Select a planet or fleet on the map or in a report.");
        }
        Selection::Planet(id) => {
            if let Some(planet) = view.planets.iter().find(|planet| planet.id == id) {
                planet_details(ui, view, planet);
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(210, 150, 80),
                    "The selected planet is no longer visible.",
                );
            }
        }
        Selection::Fleet(id) => {
            if let Some(fleet) = view.fleets.iter().find(|fleet| fleet.id == id) {
                fleet_details(ui, view, fleet);
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(210, 150, 80),
                    "The selected fleet is no longer visible.",
                );
            }
        }
    }

    ui.add_space(10.0);
    ui.separator();
    empire_summary(ui, view);
}

fn planet_details(ui: &mut egui::Ui, view: &PlayerView, planet: &PlanetView) {
    ui.heading(&planet.name);
    egui::Grid::new("planet_inspector_grid")
        .num_columns(2)
        .spacing([12.0, 3.0])
        .show(ui, |ui| {
            ui.label("Identifier");
            ui.label(planet.id.to_string());
            ui.end_row();

            ui.label("Owner");
            ui.label(owner_label(view, planet.owner));
            ui.end_row();

            ui.label("Position");
            ui.label(format!("{}, {}", planet.position.x, planet.position.y));
            ui.end_row();

            ui.label("Environment");
            ui.label(format!(
                "G {} | T {} | R {}",
                planet.environment.gravity,
                planet.environment.temperature,
                planet.environment.radiation
            ));
            ui.end_row();

            ui.label("Concentration");
            ui.label(format!(
                "I {} | B {} | G {}",
                planet.mineral_concentration[0],
                planet.mineral_concentration[1],
                planet.mineral_concentration[2]
            ));
            ui.end_row();

            ui.label("Surface minerals");
            ui.label(format!(
                "{} / {} / {}",
                format_integer(planet.surface_minerals[0]),
                format_integer(planet.surface_minerals[1]),
                format_integer(planet.surface_minerals[2])
            ));
            ui.end_row();

            ui.label("Population");
            ui.label(format_integer(planet.population));
            ui.end_row();

            ui.label("Installations");
            ui.label(format!(
                "{} factories | {} mines | {} defenses",
                planet.factories, planet.mines, planet.defenses
            ));
            ui.end_row();

            ui.label("Status");
            ui.label(match (planet.homeworld, planet.starbase) {
                (true, true) => "Homeworld with starbase",
                (true, false) => "Homeworld",
                (false, true) => "Starbase present",
                (false, false) => "No starbase",
            });
            ui.end_row();
        });
}

fn fleet_details(ui: &mut egui::Ui, view: &PlayerView, fleet: &FleetView) {
    ui.heading(&fleet.name);
    egui::Grid::new("fleet_inspector_grid")
        .num_columns(2)
        .spacing([12.0, 3.0])
        .show(ui, |ui| {
            ui.label("Identifier");
            ui.label(fleet.id.to_string());
            ui.end_row();

            ui.label("Owner");
            ui.label(owner_label(view, Some(fleet.owner)));
            ui.end_row();

            ui.label("Ships");
            ui.label(fleet.ships.to_string());
            ui.end_row();

            ui.label("Design");
            ui.label(design_label(view, fleet));
            ui.end_row();

            ui.label("Location");
            ui.label(location_label(view, fleet));
            ui.end_row();
        });
}

fn empire_summary(ui: &mut egui::Ui, view: &PlayerView) {
    egui::CollapsingHeader::new("Empire and race")
        .default_open(true)
        .show(ui, |ui| {
            match &view.race {
                Some(race) => {
                    ui.label(format!("{} ({})", race.name, race.plural_name));
                    ui.label(format!(
                        "Primary: {} ({})",
                        race.primary.name(),
                        race.primary.abbreviation()
                    ));
                    if race.lesser.is_empty() {
                        ui.label("Lesser traits: none");
                    } else {
                        let names = race
                            .lesser
                            .iter()
                            .map(|trait_value| {
                                format!("{} ({})", trait_value.name(), trait_value.abbreviation())
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        ui.label(format!("Lesser traits: {names}"));
                    }
                    ui.label(format!(
                        "Starting technology: E{} W{} P{} C{} El{} B{}",
                        race.starting_technology[0],
                        race.starting_technology[1],
                        race.starting_technology[2],
                        race.starting_technology[3],
                        race.starting_technology[4],
                        race.starting_technology[5]
                    ));
                }
                None => {
                    ui.label("No race is attached to this player slot.");
                }
            }

            let owned = view
                .planets
                .iter()
                .filter(|planet| planet.owner == Some(view.player))
                .count();
            let fleets = view
                .fleets
                .iter()
                .filter(|fleet| fleet.owner == view.player)
                .count();
            ui.label(format!("Visible empire: {owned} planets, {fleets} fleets"));
            ui.label(format!(
                "Readiness: {}",
                if view.ready { "ready" } else { "not ready" }
            ));
        });
}

fn role_label(role: PlayerRole) -> &'static str {
    match role {
        PlayerRole::Human => "human",
        PlayerRole::Computer => "computer",
    }
}
