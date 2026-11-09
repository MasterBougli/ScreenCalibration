#![windows_subsystem = "windows"]

mod app;
mod config;
mod display;
mod gamma;
mod pattern_window;
mod patterns;

use eframe::egui;

/// Mode silencieux lancé à l'ouverture de session : réapplique les corrections enregistrées.
fn apply_saved() {
    let config = config::Config::load();
    for monitor in display::enumerate() {
        if let Some(cfg) = config.monitors.get(&monitor.gdi_name) {
            if cfg.apply_at_startup && !monitor.hdr_active {
                let _ = gamma::apply(&monitor.gdi_name, &cfg.gamma);
            }
        }
    }
}

fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--apply") {
        apply_saved();
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Calibration écran")
            .with_inner_size([1100.0, 760.0])
            .with_min_inner_size([800.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native("Calibration écran", options, Box::new(|_cc| Ok(Box::new(app::App::new()))))
}
