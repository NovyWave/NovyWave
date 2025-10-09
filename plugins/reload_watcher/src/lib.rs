mod bindings {
    wit_bindgen::generate!({
        path: "../wit",
        world: "runtime",
    });
}

use bindings::{__export_world_runtime_cabi, host, Guest};

struct ReloadWatcher;

fn configure_watchers() {
    let opened = host::get_opened_files();
    let debounce_ms = 250u32;
    host::register_watched_files(&opened, debounce_ms);
    host::log_info(&format!(
        "Registered {} waveform path(s) for live reload",
        opened.len()
    ));
}

impl Guest for ReloadWatcher {
    fn init() {
        configure_watchers();
    }

    fn greet() {
        configure_watchers();
    }

    fn shutdown() {
        host::log_info("Reload watcher shutting down");
        host::clear_watched_files();
    }
}

__export_world_runtime_cabi!(ReloadWatcher with_types_in bindings);
