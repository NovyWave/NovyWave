//! NovyWave Main Entry Point

use std::sync::OnceLock;
use zoon::*;

/// Stores the main application task handle to prevent it from being dropped.
static MAIN_TASK: OnceLock<TaskHandle> = OnceLock::new();

// Core modules
mod app;
mod clipboard;
mod config;
mod connection;
mod error_display;
mod error_ui;
mod platform;
mod selected_variables;
mod test_api;
mod tracked_files;
mod virtual_list;
mod visualizer;

mod action_buttons;
mod dragging;
mod file_management;
mod file_operations;
mod file_picker;
mod format_selection;
mod panel_layout;
mod selected_variables_panel;
mod signal_processing;
mod variable_selection_ui;

/// Main application layout function
///
/// Implements dock-responsive 3-panel layout as specified:
/// - Default (dock to bottom): Files & Scopes + Variables (top row), Selected Variables (bottom)
/// - Dock to right: Files & Scopes over Variables (left column), Selected Variables (right)
pub fn main_layout(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
    dragging_system: &crate::dragging::DraggingSystem,
    waveform_canvas: &crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
    file_dialog_visible: &zoon::Mutable<bool>,
) -> impl Element {
    use crate::file_management::files_panel_with_dialog;
    use crate::variable_selection_ui::variables_panel_with_fill;

    El::new().s(Width::fill()).s(Height::fill()).child_signal(
        app_config.dock_mode.signal_cloned().map({
            let tracked_files = tracked_files.clone();
            let selected_variables = selected_variables.clone();
            let waveform_timeline = waveform_timeline.clone();
            let app_config = app_config.clone();
            let dragging_system = dragging_system.clone();
            let waveform_canvas = waveform_canvas.clone();
            let file_dialog_visible = file_dialog_visible.clone();

            move |dock_mode| {
                match dock_mode {
                    // Default layout: Files & Variables (top row), Selected Variables (bottom)
                    shared::DockMode::Bottom => {
                        let top_section_height_signal =
                            crate::dragging::files_panel_height_signal(app_config.clone());

                        El::new().s(Width::fill()).s(Height::fill()).child(
                        Column::new()
                            .s(Width::fill())
                            .s(Height::fill())
                            .item(
                                El::new()
                                    .s(Width::fill())
                                    .s(Height::exact_signal(
                                        top_section_height_signal.map(|h| h as u32),
                                    ))
                                    .child(
                                        Row::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .item(
                                                El::new()
                                                    .s(Height::fill())
                                                    .s(Width::exact_signal(
                                                        crate::dragging::files_panel_width_signal(
                                                            app_config.clone(),
                                                        )
                                                        .map(|w| w as u32),
                                                    ))
                                                    .child(files_panel_with_dialog(
                                                        tracked_files.clone(),
                                                        selected_variables.clone(),
                                                        file_dialog_visible.clone(),
                                                        app_config.clone(),
                                                    )),
                                            )
                                            .item(
                                                crate::panel_layout::files_panel_vertical_divider(
                                                    &app_config,
                                                    dragging_system.clone(),
                                                ),
                                            )
                                            .item(
                                                El::new().s(Width::fill()).s(Height::fill()).child(
                                                    variables_panel_with_fill(
                                                        &tracked_files,
                                                        &selected_variables,
                                                        &waveform_timeline,
                                                        &waveform_canvas,
                                                        &app_config,
                                                    ),
                                                ),
                                            ),
                                    ),
                            )
                            .item(crate::panel_layout::files_panel_horizontal_divider(
                                &app_config,
                                dragging_system.clone(),
                            ))
                            .item(El::new().s(Width::fill()).s(Height::fill()).child(
                                crate::selected_variables_panel::selected_variables_panel(
                                    selected_variables.clone(),
                                    waveform_timeline.clone(),
                                    tracked_files.clone(),
                                    app_config.clone(),
                                    dragging_system.clone(),
                                    waveform_canvas.clone(),
                                ),
                            )),
                    )
                    }

                    // Right dock layout: Files over Variables (left), Selected Variables (right)
                    shared::DockMode::Right => El::new().s(Width::fill()).s(Height::fill()).child(
                        Row::new()
                            .s(Width::fill())
                            .s(Height::fill())
                            .item(
                                El::new()
                                    .s(Height::fill())
                                    .s(Width::exact_signal(
                                        crate::dragging::files_panel_width_signal(
                                            app_config.clone(),
                                        )
                                        .map(|w| w as u32),
                                    ))
                                    .child(
                                        Column::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .item(
                                                El::new()
                                                    .s(Width::fill())
                                                    .s(Height::exact_signal(
                                                        crate::dragging::files_panel_height_signal(
                                                            app_config.clone(),
                                                        )
                                                        .map(|h| h as u32),
                                                    ))
                                                    .child(files_panel_with_dialog(
                                                        tracked_files.clone(),
                                                        selected_variables.clone(),
                                                        file_dialog_visible.clone(),
                                                        app_config.clone(),
                                                    )),
                                            )
                                            .item(
                                                crate::panel_layout::files_panel_horizontal_divider(
                                                    &app_config,
                                                    dragging_system.clone(),
                                                ),
                                            )
                                            .item(
                                                El::new().s(Width::fill()).s(Height::fill()).child(
                                                    variables_panel_with_fill(
                                                        &tracked_files,
                                                        &selected_variables,
                                                        &waveform_timeline,
                                                        &waveform_canvas,
                                                        &app_config,
                                                    ),
                                                ),
                                            ),
                                    ),
                            )
                            .item(crate::panel_layout::files_panel_vertical_divider(
                                &app_config,
                                dragging_system.clone(),
                            ))
                            .item(El::new().s(Width::fill()).s(Height::fill()).child(
                                crate::selected_variables_panel::selected_variables_panel(
                                    selected_variables.clone(),
                                    waveform_timeline.clone(),
                                    tracked_files.clone(),
                                    app_config.clone(),
                                    dragging_system.clone(),
                                    waveform_canvas.clone(),
                                ),
                            )),
                    ),
                }
            }
        }),
    )
}

pub fn main() {
    // Tauri builds don't include the devserver's ReconnectingEventSource helper.
    // Provide a minimal shim so zoon's Connection can initialize without crashing.
    ensure_reconnecting_event_source();

    let handle = Task::start_droppable(async {
        let app = crate::app::NovyWaveApp::new().await;

        // Store components for test API access
        test_api::store_test_api_state(
            app.tracked_files.clone(),
            app.selected_variables.clone(),
            app.waveform_timeline.clone(),
            app.connection.clone(),
        );
        test_api::expose_novywave_test_api();

        let root_element = app.root();
        start_app("app", move || root_element);
    });
    let _ = MAIN_TASK.set(handle);
}

#[cfg(not(NOVYWAVE_PLATFORM = "TAURI"))]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function ensure_reconnecting_event_source() {
  if (typeof window === 'undefined') return;
  if (typeof window.ReconnectingEventSource !== 'undefined') return;
  if (typeof window.EventSource !== 'undefined') {
    window.ReconnectingEventSource = window.EventSource;
    return;
  }
  // Fallback stub: won't stream events but prevents init crash.
  window.ReconnectingEventSource = function(url) {
    console.warn('ReconnectingEventSource stub: EventSource not available', url);
    this.url = url;
    this.close = function() {};
    this.addEventListener = function() {};
    this.removeEventListener = function() {};
    this.dispatchEvent = function() { return true; };
  };
}
"#)]
extern "C" {
    fn ensure_reconnecting_event_source();
}

#[cfg(NOVYWAVE_PLATFORM = "TAURI")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function ensure_reconnecting_event_source() {
  if (typeof window === 'undefined') return;
  if (typeof window.EventSource === 'undefined') return;

  const ORIGIN = 'http://127.0.0.1:8080';

  const rewriteApi = (urlStr) => {
    try {
      const url = new URL(urlStr, ORIGIN);
      const path = url.pathname;
      const is_api =
        path.startsWith('/_api/message') ||
        path.startsWith('/_api/message_sse') ||
        path.startsWith('/_api/load') ||
        path.startsWith('/_api/browse');
      if (!is_api) return urlStr;
      url.protocol = 'http:';
      url.host = '127.0.0.1:8080';
      return url.toString();
    } catch (_e) {
      return urlStr;
    }
  };

  // Log all fetch calls to trace UpMsg POSTs.
  const origFetch = window.fetch.bind(window);
  window.fetch = (input, init) => {
    console.log("[fetch] request", input, init);
    try {
      if (typeof input === 'string') {
        const rewritten = rewriteApi(input);
        if (rewritten !== input) {
          input = rewritten;
          console.log("[fetch] rewritten ->", input);
        }
      } else if (input && input.url) {
        const rewritten = rewriteApi(input.url);
        if (rewritten !== input.url) {
          input = new Request(rewritten, input);
          console.log("[fetch] rewritten Request ->", rewritten);
        }
      }
    } catch (e) {
      console.log("[fetch] rewrite error", e);
    }
    return origFetch(input, init)
      .then((resp) => {
        console.log("[fetch] response", resp.url, resp.status);
        return resp;
      })
      .catch((err) => {
        console.log("[fetch] error", err);
        throw err;
      });
  };

  const NativeEventSource = window.EventSource;
  window.EventSource = function(url, opts) {
    const rewritten = rewriteApi(url);
    console.log("[ES] constructing EventSource", url, "->", rewritten);
    const es = new NativeEventSource(rewritten, opts);
    es.addEventListener('open', () => console.log("[ES] open", url));
    es.addEventListener('error', (e) => console.log("[ES] error", url, e));
    return es;
  };
  window.EventSource.prototype = NativeEventSource.prototype;

  // Provide ReconnectingEventSource symbol (no reconnection logic needed here)
  window.ReconnectingEventSource = window.EventSource;
}
"#)]
extern "C" {
    fn ensure_reconnecting_event_source();
}

// Rebuild trigger
