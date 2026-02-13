mod api;
mod audio;
mod config;
mod db;
mod input;
mod ui;

use gtk4::prelude::*;
use std::sync::Arc;

fn main() {
    let config = Arc::new(config::Config::load());

    let app = gtk4::Application::builder()
        .application_id("dev.linwhisper.app")
        .build();

    let config_c = Arc::clone(&config);
    app.connect_activate(move |app| {
        ui::build_ui(app, Arc::clone(&config_c));
    });

    app.run();
}
