use zoon::Mutable;
use crate::file_operations::extract_filename;
use futures::StreamExt;
use moonzoon_novyui::tokens::color::{
    neutral_1, neutral_2, neutral_4, neutral_8, neutral_11, neutral_12,
};
#[cfg(NOVYWAVE_PLATFORM = "WEB")]
use moonzoon_novyui::tokens::color::{primary_3, primary_6};
use moonzoon_novyui::tokens::theme::Theme;
use moonzoon_novyui::*;
use shared::FileSystemItem;
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use zoon::events::{Click, KeyDown};
use zoon::map_ref;
use zoon::*;



#[cfg(NOVYWAVE_PLATFORM = "WEB")]
fn apply_scrollbar_colors(
    raw_el: zoon::RawHtmlEl<web_sys::HtmlElement>,
) -> zoon::RawHtmlEl<web_sys::HtmlElement> {
    raw_el.style_signal(
        "scrollbar-color",
        primary_6()
            .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
            .flatten(),
    )
}

#[cfg(not(NOVYWAVE_PLATFORM = "WEB"))]
fn apply_scrollbar_colors(
    raw_el: zoon::RawHtmlEl<web_sys::HtmlElement>,
) -> zoon::RawHtmlEl<web_sys::HtmlElement> {
    raw_el
}



/// Initialize directories on first use and when restored from config
pub fn initialize_directories_and_request_contents(
    file_picker_domain: &crate::config::FilePickerDomain,
) {
    // Read current state directly - no async needed for Mutable reads
    let cache = file_picker_domain.directory_cache.get_cloned();
    let expanded = file_picker_domain.expanded_directories.get_cloned();

    if !cache.contains_key("/") {
        file_picker_domain.expand_directory("/".to_string());
    }

    if expanded.is_empty() {
        if let Some(home_dir) = std::env::var("HOME")
            .ok()
            .or_else(|| std::env::var("USERPROFILE").ok())
        {
            file_picker_domain.expand_directory("/".to_string());
            file_picker_domain.expand_directory(home_dir.clone());
        } else {
            file_picker_domain.expand_directory("/".to_string());
        }
    } else {
        // Request contents only for directories not already in cache
        for directory in &expanded {
            if !cache.contains_key(directory) {
                file_picker_domain.expand_directory(directory.clone());
            }
        }
    }
}

/// Build tree data for TreeView component using Actor signals
fn build_tree_data(
    root_path: &str,
    cache: &HashMap<String, Vec<FileSystemItem>>,
    errors: &HashMap<String, String>,
) -> Vec<TreeViewItemData> {
    let mut root_items = Vec::new();

    if let Some(items) = cache.get(root_path) {
        for item in items {
            if item.is_directory {
                root_items.push(build_directory_item(&item.path, &item.name, cache, errors));
            } else if item.is_waveform_file {
                root_items.push(
                    TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("file".to_string())
                        .item_type(TreeViewItemType::File)
                        .is_waveform_file(true),
                );
            }
        }
    }

    root_items
}

/// Build directory tree item with children
fn build_directory_item(
    path: &str,
    name: &str,
    cache: &HashMap<String, Vec<shared::FileSystemItem>>,
    errors: &HashMap<String, String>,
) -> TreeViewItemData {
    let mut item = TreeViewItemData::new(path.to_string(), name.to_string())
        .icon("folder".to_string())
        .item_type(TreeViewItemType::Folder);

    if let Some(_error) = errors.get(path) {
        // Directory has access error
        item = item.with_children(vec![
            TreeViewItemData::new(
                "error".to_string(),
                "Can't access this directory".to_string(),
            )
            .disabled(true)
            .item_type(TreeViewItemType::Default),
        ]);
    } else if cache.contains_key(path) {
        let child_items = build_tree_data(path, cache, errors);

        if child_items.is_empty() {
            // Directory is empty
            item = item.with_children(vec![
                TreeViewItemData::new("empty".to_string(), "Empty".to_string())
                    .disabled(true)
                    .item_type(TreeViewItemType::Default),
            ]);
        } else {
            item = item.with_children(child_items);
        }
    } else {
        item = item.with_children(vec![
            TreeViewItemData::new("loading".to_string(), "Loading...".to_string())
                .disabled(true)
                .item_type(TreeViewItemType::Default),
        ]);
    }

    item
}

/// Main file paths dialog for selecting waveform files
pub fn file_paths_dialog(
    tracked_files: crate::tracked_files::TrackedFiles,
    _selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
    file_dialog_visible: Mutable<bool>,
    connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    // Count files that are actually in loading state, not total files
    let loading_count_broadcaster = tracked_files
        .files
        .signal_vec_cloned()
        .to_signal_cloned()
        .map(move |files| {
            files
                .iter()
                .filter(|file| matches!(file.state, shared::FileState::Loading(_)))
                .count()
        })
        .broadcast();

    // Get file picker domain for proper selected files management
    let file_picker_domain = app_config.file_picker_domain.clone();

    let close_dialog = {
        let file_dialog_visible = file_dialog_visible.clone();
        move || file_dialog_visible.set(false)
    };

    El::new()
        .s(Background::new().color_signal(theme().map(|t| match t {
            Theme::Light => "rgba(255, 255, 255, 0.8)",  // Light overlay
            Theme::Dark => "rgba(0, 0, 0, 0.8)",          // Dark overlay
        })))
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .s(Padding::all(40))
        .update_raw_el(|raw_el| {
            raw_el
                .style("display", "flex")
                .style("position", "fixed")
                .style("inset", "0")
                .style("z-index", "20000")
                .style("justify-content", "center")
                .style("align-items", "center")
        })
        .update_raw_el({
            let close_dialog = close_dialog.clone();
            let tracked_files_for_enter = tracked_files.clone();
            let file_picker_domain_for_enter = file_picker_domain.clone();
            let file_dialog_visible_for_enter = file_dialog_visible.clone();
            move |raw_el| {
                raw_el
                    .event_handler({
                        let close_dialog = close_dialog.clone();
                        move |_event: Click| {
                            close_dialog();
                        }
                    })
                    .global_event_handler({
                        let tracked_files_for_enter = tracked_files_for_enter.clone();
                        let file_picker_domain_for_enter = file_picker_domain_for_enter.clone();
                        let file_dialog_visible_for_enter = file_dialog_visible_for_enter.clone();
                        let close_dialog = close_dialog.clone();
                        move |event: KeyDown| {
                            if event.key() == "Escape" {
                                close_dialog();
                            } else if event.key() == "Enter" {
                                // Get the selected files and load them
                                let selected_files_value = file_picker_domain_for_enter.selected_files_vec_signal.get_cloned();
                                if !selected_files_value.is_empty() {
                                    zoon::println!("🎯 Loading {} selected files via Enter key", selected_files_value.len());
                                    crate::file_operations::process_file_picker_selection(
                                        &tracked_files_for_enter,
                                        selected_files_value,
                                        &file_dialog_visible_for_enter
                                    );
                                    // Clear the selection after loading files so dialog is ready for next selection
                                    file_picker_domain_for_enter.clear_file_selection();
                                }
                            }
                        }
                    })
            }
        })
        .child(
            El::new()
                .s(Background::new().color_signal(neutral_2()))
                .s(RoundedCorners::all(8))
                .s(Borders::all_signal(neutral_4().map(|color| {
                    Border::new().width(1).color(color)
                })))
                .s(Padding::all(16))
                .s(Width::fill().min(500).max(600))
                .s(Height::fill().max(800))
                .update_raw_el(|raw_el| {
                    raw_el
                        .event_handler(|event: Click| {
                            event.stop_propagation();
                        })
                })
                .child(
                    Column::new()
                        .s(Height::fill())
                        .s(Gap::new().y(SPACING_16))
                        .item(
                            Row::new()
                                .s(Gap::new().x(SPACING_4))
                                .item(
                                    El::new()
                                        .s(Font::new().size(16).weight(FontWeight::Bold).color_signal(neutral_12()))
                                        .child("Select Waveform Files ")
                                )
                                .item(
                                    El::new()
                                        .s(Font::new().size(16).weight(FontWeight::Bold).color_signal(neutral_8()))
                                        .child("(*.vcd, *.fst, *.ghw)")
                                )
                        )
                        .item(
                            El::new()
                                .s(Height::fill())
                                .s(Background::new().color_signal(neutral_1()))
                                .s(Borders::all_signal(neutral_4().map(|color| {
                                    Border::new().width(1).color(color)
                                })))
                                .s(RoundedCorners::all(4))
                                .s(Padding::all(8))
                                .update_raw_el(|raw_el| {
                                    raw_el
                                        .style("min-height", "0")      // Allow flex shrinking
                                        .style("overflow-x", "auto")   // Enable horizontal scroll
                                        .style("overflow-y", "hidden") // Prevent double scrollbars
                                })
                                .child(file_picker_content(&app_config, connection))
                        )
                        .item(
                            El::new()
                                .s(Padding::all(4))
                                .child_signal({
                                    let file_picker_domain_for_len = file_picker_domain.clone();
                                    let file_picker_domain_for_tags = file_picker_domain.clone();
                                    file_picker_domain_for_len.selected_files.signal_vec_cloned().len().map(move |selected_count| {
                                        if selected_count == 0 {
                                            El::new()
                                                .s(Font::new().italic().color_signal(neutral_8()))
                                                .child("Select waveform files from the directory tree above")
                                                .unify()
                                        } else {
                                            El::new()
                                                .child_signal({
                                                    let file_picker_domain_for_inner = file_picker_domain_for_tags.clone();
                                                    file_picker_domain_for_tags.selected_files_vec_signal.signal_cloned().map(move |files| {
                                                        let file_picker_domain_for_items = file_picker_domain_for_inner.clone();
                                                        Row::new()
                                                            .s(Gap::new().x(8).y(8))
                                                            .s(Width::fill())
                                                            .update_raw_el(|raw_el| raw_el.style("flex-wrap", "wrap"))
                                                            .items(
                                                                files.iter().map(|path| {
                                                                    let file_picker_domain_for_tag = file_picker_domain_for_items.clone();
                                                                    Row::new()
                                                                        .s(Background::new().color_signal(neutral_3()))
                                                                        .s(Padding::new().x(12).y(6))
                                                                        .s(RoundedCorners::all_max())
                                                                        .s(Gap::new().x(8))
                                                                        .s(Align::new().center_y())
                                                                        .s(Font::new().size(14).color_signal(neutral_11()))
                                                                        .update_raw_el({
                                                                            let path_for_tooltip = path.clone();
                                                                            move |raw_el| {
                                                                                raw_el.attr("title", &path_for_tooltip)
                                                                            }
                                                                        })
                                                                        .item(Text::new(&extract_filename(&path)))
                                                                        .item(
                                                                            Button::new()
                                                                                .s(Background::new().color_signal(neutral_5()))
                                                                                .s(RoundedCorners::all_max())
                                                                                .s(Width::exact(20))
                                                                                .s(Height::exact(20))
                                                                                .s(Padding::all(2))
                                                                                .s(Font::new().size(12).color_signal(neutral_11()))
                                                                                .label("✕")
                                                                                .on_press({
                                                                                    let path = path.to_string();
                                                                                    move || {
                                                                                        zoon::println!("🗑️ Badge X clicked for: {}", path);
                                                                                        file_picker_domain_for_tag.deselect_file(path.clone());
                                                                                    }
                                                                                })
                                                                        )
                                                                }).collect::<Vec<_>>()
                                                            )
                                                    })
                                                })
                                                .unify()
                                        }
                                    })
                                })
                        )
                        .item(
                            Row::new()
                                .s(Gap::new().x(SPACING_12))
                                .s(Align::new().right())
                                .item(
                                    button()
                                        .label("Cancel")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Small)
                                        .on_press(close_dialog)
                                        .build()
                                )
                                .item(
                                    button()
                                        .label_signal({
                                            let file_picker_domain_for_button = file_picker_domain.clone();
                                            map_ref! {
                                                let loading_count = loading_count_broadcaster.signal(),
                                                let selected_count = file_picker_domain_for_button.selected_files.signal_vec_cloned().len() =>
                                                move {
                                                    let is_loading = *loading_count > 0;
                                                    if is_loading {
                                                        "Loading...".to_string()
                                                    } else if *selected_count > 0 {
                                                        format!("Load {} Files", *selected_count)
                                                    } else {
                                                        "Load Files".to_string()
                                                    }
                                                }
                                            }
                                        })
                                        .disabled_signal({
                                            let file_picker_domain_for_disabled = file_picker_domain.clone();
                                            map_ref! {
                                                let loading_count = loading_count_broadcaster.signal(),
                                                let selected_count = file_picker_domain_for_disabled.selected_files.signal_vec_cloned().len() => {
                                                let is_loading = *loading_count > 0;
                                                is_loading || *selected_count == 0
                                                }
                                            }
                                        })
                                        .on_press({
                                            let tracked_files_for_press = tracked_files.clone();
                                            let file_picker_domain_for_press = file_picker_domain.clone();
                                            let file_dialog_visible_for_press = file_dialog_visible.clone();
                                            move || {
                                                // Get the actual selected files from the FilePickerDomain
                                                let selected_files_value = file_picker_domain_for_press.selected_files_vec_signal.get_cloned();
                                                zoon::println!("🎯 Loading {} selected files", selected_files_value.len());
                                                crate::file_operations::process_file_picker_selection(
                                                    &tracked_files_for_press,
                                                    selected_files_value,
                                                    &file_dialog_visible_for_press
                                                );
                                                // Clear the selection after loading files so dialog is ready for next selection
                                                file_picker_domain_for_press.clear_file_selection();
                                            }
                                        })
                                        .variant(ButtonVariant::Primary)
                                        .build()
                                )
                        )
                )
        )
}

/// File picker content with directory tree
pub fn file_picker_content(
    app_config: &crate::config::AppConfig,
    connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    let file_picker_domain = app_config.file_picker_domain.clone();

    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        // Scroll position restoration with tree rendering coordination
        .viewport_y_signal({
            let scroll_position_actor = file_picker_domain.scroll_position.clone();

            // Use map_ref! for simpler signal coordination without complex async block
            zoon::map_ref! {
                let position = scroll_position_actor.signal() => {
                    *position
                }
            }
        })
        // Scroll position saving with raw DOM event handling
        .update_raw_el({
            let domain = file_picker_domain.clone();
            move |raw_el| {
                // Use the same pattern as virtual_list.rs for raw DOM scroll events
                let html_el = raw_el.dom_element();
                let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                    move |_event: web_sys::Event| {
                        if let Some(target) = _event.current_target() {
                            if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                                let scroll_top = element.scroll_top();
                                domain.set_scroll_position(scroll_top);
                            }
                        }
                    }
                })
                    as Box<dyn FnMut(_)>);

                html_el
                    .add_event_listener_with_callback(
                        "scroll",
                        scroll_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                scroll_closure.forget();
                raw_el
            }
        })
        .update_raw_el(|raw_el| {
            let dom_element = raw_el.dom_element();
            dom_element
                .set_attribute("data-scroll-container", "file-picker")
                .unwrap();
            raw_el
                .style("min-height", "0") // Allow flex shrinking
                .style("scrollbar-width", "thin")
                .apply(|raw_el| apply_scrollbar_colors(raw_el))
        })
        .child(file_picker_tree(
            &app_config,
            connection,
        ))
}

/// File picker tree using FilePickerDomain Actors
pub fn file_picker_tree(
    app_config: &crate::config::AppConfig,
    _connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    // Initialize directories and request contents for expanded directories from config
    initialize_directories_and_request_contents(&app_config.file_picker_domain);

    let domain_for_treeview = app_config.file_picker_domain.clone();
    let cache_signal = app_config.file_picker_domain.directory_cache_signal();

    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .update_raw_el(|raw_el| {
            raw_el
                .style("min-width", "fit-content")
                .style("width", "100%")
                .style("scrollbar-width", "thin")
                .apply(|raw_el| apply_scrollbar_colors(raw_el))
        })
        .child_signal({
            let initialization_actor_for_closure = initialization_actor.clone();
            cache_signal.map(move |cache| {
                if cache.contains_key("/") {
                    // Root directory loaded - show tree
                    let tree_data = build_tree_data("/", &cache, &std::collections::HashMap::new());

                    {
                        // Single Source of Truth: TreeView uses domain's Mutable directly
                        // Detection task in FilePickerDomain handles browse/save side effects
                        let external_expanded = domain_for_treeview.expanded_directories.clone();

                        // Store initialization actor for proper lifecycle management
                        let _initialization_actor = initialization_actor_for_closure.clone();

                        let scroll_task_handle: std::sync::Arc<std::sync::Mutex<Option<zoon::TaskHandle>>> =
                            std::sync::Arc::new(std::sync::Mutex::new(None));
                        let scroll_task_handle_for_insert = scroll_task_handle.clone();
                        let scroll_task_handle_for_remove = scroll_task_handle.clone();

                        El::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .after_remove(move |_| {
                                drop(_initialization_actor);
                                drop(scroll_task_handle_for_remove.lock().unwrap().take());
                            })
                            // Restore scroll position after tree is rendered
                            .after_insert({
                                let scroll_position_actor =
                                    domain_for_treeview.scroll_position.clone();
                                move |_element| {
                                    // Use sync .get() to get current position
                                    let position = scroll_position_actor.get();

                                    // Defer scroll restoration to next macro tick
                                    let handle = zoon::Task::start_droppable(async move {
                                        zoon::Task::next_macro_tick().await;

                                        if position > 0 {
                                            if let Some(window) = web_sys::window() {
                                                if let Some(document) = window.document() {
                                                    if let Ok(Some(element)) = document
                                                        .query_selector(
                                                            "[data-scroll-container='file-picker']",
                                                        )
                                                    {
                                                        element.set_scroll_top(position);
                                                    }
                                                }
                                            }
                                        }
                                    });
                                    *scroll_task_handle_for_insert.lock().unwrap() = Some(handle);
                                }
                            })
                            .child(
                                tree_view()
                                    .data(tree_data)
                                    .size(TreeViewSize::Medium)
                                    .variant(TreeViewVariant::Basic)
                                    .show_icons(true)
                                    .show_checkboxes(true)
                                    .external_expanded(external_expanded)
                                    // Single Source of Truth: TreeView uses domain's MutableVec directly
                                    .external_selected_vec(domain_for_treeview.selected_files.clone())
                                    .build()
                                    .into_raw(),
                            )
                            .into_element()
                    }
                } else {
                    // Still loading root directory
                    El::new()
                        .s(Padding::all(20))
                        .s(Font::new().color_signal(neutral_8()).italic())
                        .child("Loading directory contents...")
                        .into_element()
                }
            })
        })
}
