use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_4, neutral_8, neutral_11, neutral_12, primary_3, primary_6};
use moonzoon_novyui::tokens::theme::Theme;
use moonzoon_novyui::*;
use zoon::events::{Click, KeyDown};
use zoon::*;
use crate::dataflow::{atom::Atom, relay};
use std::collections::{HashMap, HashSet};

/// Main file paths dialog for selecting waveform files
pub fn file_paths_dialog(
    tracked_files: crate::tracked_files::TrackedFiles,
    _selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
    file_dialog_visible: Atom<bool>
) -> impl Element {
    let file_count_broadcaster = tracked_files.files_vec_signal.signal_cloned().map(|files| files.len()).broadcast();
    
    let selected_files = zoon::Mutable::new(Vec::<String>::new());

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
                .style("justify-content", "center")
                .style("align-items", "center")
        })
        .update_raw_el({
            let close_dialog = close_dialog.clone();
            move |raw_el| {
                raw_el
                    .event_handler({
                        let close_dialog = close_dialog.clone();
                        move |_event: Click| {
                            close_dialog();
                        }
                    })
                    .global_event_handler({
                        move |event: KeyDown| {
                            if event.key() == "Escape" {
                                close_dialog();
                            } else if event.key() == "Enter" {
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
                                        .child("(*.vcd, *.fst)")
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
                                .child(file_picker_content(&app_config, &selected_files))
                        )
                        .item(
                            El::new()
                                .s(Padding::all(4))
                                .child_signal(
                                    selected_files.signal_cloned().map(|selected_paths| {

                                        if selected_paths.is_empty() {
                                            El::new()
                                                .s(Font::new().italic().color_signal(neutral_8()))
                                                .child("Select waveform files from the directory tree above")
                                                .unify()
                                        } else {
                                            Row::new()
                                                .multiline()
                                                .s(Gap::new().x(SPACING_8).y(SPACING_8))
                                                .s(Align::new().left().top())
                                                .items(selected_paths.iter().map(|path| {
                                                    let filename = crate::file_operations::extract_filename(path);
                                                    badge(filename)
                                                        .variant(BadgeVariant::Outline)
                                                        .size(BadgeSize::Small)
                                                        .removable()
                                                        .on_remove({
                                                            let path = path.clone();
                                                            move || {
                                                                zoon::println!("Remove file: {}", path);
                                                            }
                                                        })
                                                        .build()
                                                }))
                                                .unify()
                                        }
                                    })
                                )
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
                                        .label_signal(
                                            map_ref! {
                                                let file_count = file_count_broadcaster.signal(),
                                                let selected_files = selected_files.signal_cloned() =>
                                                move {
                                                    let is_loading = *file_count > 0;
                                                    let selected_count = selected_files.len();
                                                    if is_loading {
                                                        "Loading...".to_string()
                                                    } else if selected_count > 0 {
                                                        format!("Load {} Files", selected_count)
                                                    } else {
                                                        "Load Files".to_string()
                                                    }
                                                }
                                            }
                                        )
                                        .disabled_signal(
                                            map_ref! {
                                                let file_count = file_count_broadcaster.signal(),
                                                let selected_files = selected_files.signal_cloned() => {
                                                let is_loading = *file_count > 0;
                                                let selected_count = selected_files.len();
                                                is_loading || selected_count == 0
                                                }
                                            }
                                        )
                                        .on_press({
                                            let tracked_files_for_press = tracked_files.clone();
                                            let selected_files_for_press = selected_files.clone();
                                            let file_dialog_visible_for_press = file_dialog_visible.clone();
                                            move || {
                                                let selected_files_value = selected_files_for_press.get_cloned();
                                                crate::file_operations::process_file_picker_selection(
                                                    tracked_files_for_press.clone(),
                                                    selected_files_value,
                                                    file_dialog_visible_for_press.clone()
                                                );
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
    selected_files: &zoon::Mutable<Vec<String>>,
) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin").style_signal(
                "scrollbar-color",
                primary_6()
                    .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
                    .flatten(),
            )
        })
        .child_signal(simple_file_picker_tree(app_config.clone()).into_signal_option())
}

/// Simple file picker tree implementation
pub async fn simple_file_picker_tree(app_config: crate::config::AppConfig) -> impl Element {
    let selected_files = zoon::MutableVec::<String>::new();
    let scroll_position = app_config
        .file_picker_scroll_position
        .signal()
        .to_stream()
        .next()
        .await;
    let (tree_view_rendering_relay, mut tree_view_rendering_stream) = relay();
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Scrollbars::both())
        .viewport_y_signal(signal::from_future(Box::pin(async move {
            tree_view_rendering_stream.next().await;
            Task::next_macro_tick().await;
            scroll_position
        })).map(|position| position.flatten().unwrap_or_default()))
        .on_viewport_location_change({
            let scroll_position_mutable = app_config.file_picker_scroll_position.clone();
            move |_scene, viewport| {
                scroll_position_mutable.set_neq(viewport.y);
            }
        })
        .update_raw_el(|raw_el| {
            raw_el
                .style("min-width", "fit-content")
                .style("width", "100%")
                .style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child_signal(
            map_ref! {
                let tree_cache = zoon::always(std::collections::HashMap::new()), // Use empty cache for now
                let error_cache = zoon::always(std::collections::HashMap::new()),
                let expanded = app_config.file_picker_expanded_directories.signal_cloned() =>
                move {

                    crate::file_operations::monitor_directory_expansions(expanded.iter().cloned().collect::<HashSet<_>>(), &app_config);

                    if let Some(_root_items) = tree_cache.get("/") {
                        let tree_data = vec![
                            TreeViewItemData::new("/".to_string(), "/".to_string())
                                .with_children(build_hierarchical_tree("/", &tree_cache, &error_cache))
                        ];

                        El::new()
                            .after_insert(clone!((tree_view_rendering_relay) move |_| {
                                tree_view_rendering_relay.send(());
                            }))
                            .child(
                                tree_view()
                                    .data(tree_data)
                                    .size(TreeViewSize::Medium)
                                    .variant(TreeViewVariant::Basic)
                                    .show_icons(true)
                                    .show_checkboxes(true)
                                    .external_expanded(app_config.file_picker_expanded_directories.clone())
                                    .external_selected_vec(selected_files.clone())
                                    .build()
                            )
                            .unify()

                    } else {
                        El::new()
                            .s(Padding::all(20))
                            .s(Font::new().color_signal(neutral_8()).italic())
                            .child("Loading directory contents...")
                            .unify()
                    }
                }
            }
        )
}

/// Check if folder should be disabled based on content
pub fn should_disable_folder(
    path: &str,
    tree_cache: &HashMap<String, Vec<shared::FileSystemItem>>,
) -> bool {
    if let Some(items) = tree_cache.get(path) {
        let has_subfolders = items.iter().any(|item| item.is_directory);
        let has_waveform_files = items
            .iter()
            .any(|item| !item.is_directory && item.is_waveform_file);

        return !has_subfolders && !has_waveform_files;
    }

    false
}

/// Build hierarchical tree structure from cache data
pub fn build_hierarchical_tree(
    path: &str,
    tree_cache: &HashMap<String, Vec<shared::FileSystemItem>>,
    error_cache: &HashMap<String, String>,
) -> Vec<TreeViewItemData> {
    if let Some(items) = tree_cache.get(path) {
        items
            .iter()
            .map(|item| {
                if item.is_directory {
                    if let Some(_error_msg) = error_cache.get(&item.path) {
                        let data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                            .icon("folder".to_string())
                            .item_type(TreeViewItemType::Folder)
                            .with_children(vec![
                                TreeViewItemData::new(
                                    "access_denied",
                                    &error_cache
                                        .get(&item.path)
                                        .map(|err| crate::error_display::make_error_user_friendly(err))
                                        .unwrap_or_else(|| {
                                            "Cannot access this directory".to_string()
                                        }),
                                )
                                .item_type(TreeViewItemType::Default)
                                .disabled(true),
                            ]);
                        data
                    } else if let Some(_children) = tree_cache.get(&item.path) {
                        let children = build_hierarchical_tree(&item.path, tree_cache, error_cache);
                        let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                            .icon("folder".to_string())
                            .item_type(TreeViewItemType::Folder)
                            .with_children(children);

                        if should_disable_folder(&item.path, tree_cache) {
                            data = data.with_children(vec![
                                TreeViewItemData::new("no_supported_files", "No supported files")
                                    .item_type(TreeViewItemType::Default)
                                    .disabled(true),
                            ]);
                        }

                        data
                    } else {
                        let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                            .icon("folder".to_string())
                            .item_type(TreeViewItemType::Folder);

                        if item.has_expandable_content {
                            data = data.with_children(vec![
                                TreeViewItemData::new("loading", "Loading...")
                                    .item_type(TreeViewItemType::Default)
                                    .disabled(true),
                            ]);
                        } else {
                            data = data.with_children(vec![
                                TreeViewItemData::new("no_supported_files", "No supported files")
                                    .item_type(TreeViewItemType::Default)
                                    .disabled(true),
                            ]);
                        }

                        data
                    }
                } else {
                    let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("file".to_string())
                        .item_type(TreeViewItemType::File)
                        .is_waveform_file(item.is_waveform_file);

                    if !item.is_waveform_file {
                        data = data.disabled(true);
                    }

                    data
                }
            })
            .collect()
    } else {
        vec![]
    }
}