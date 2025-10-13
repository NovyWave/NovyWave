mod bindings {
    wit_bindgen::generate!({
        path: "./wit",
    });
}

use bindings::{__export_world_plugin_cabi, novywave::reload_watcher::host, Guest};

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

fn request_reload(paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    host::reload_waveform_files(paths);
    host::log_info(&format!(
        "Requested reload for {} waveform path(s)",
        paths.len()
    ));
}

impl Guest for ReloadWatcher {
    fn init() {
        configure_watchers();
    }

    fn refresh_opened_files() {
        configure_watchers();
    }

    fn watched_files_changed(paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        request_reload(&paths);
    }

    fn shutdown() {
        host::log_info("Reload watcher shutting down");
        host::clear_watched_files();
    }
}

__export_world_plugin_cabi!(ReloadWatcher with_types_in bindings);
