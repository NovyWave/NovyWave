use crate::dataflow::atom::Atom;
use moonzoon_novyui::tokens::color::{neutral_8, neutral_11, primary_6};
use moonzoon_novyui::*;
use shared::{FileState, ScopeData, TrackedFile, generate_smart_labels};
use std::collections::HashMap;
use std::sync::Arc;
use zoon::*;

/// Create the main files panel with proper Load Files button integration
pub fn files_panel_with_dialog(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    file_dialog_visible: Atom<bool>,
    app_config: crate::config::AppConfig,
) -> impl Element {
    let file_count_broadcaster = tracked_files.files.signal_vec().len().broadcast();
    El::new()
        .s(Height::fill())
        .child(crate::panel_layout::create_panel(
            Row::new()
                .s(Gap::new().x(SPACING_8))
                .s(Align::new().center_y())
                .item(El::new().s(Font::new().no_wrap()).child("Files & Scopes"))
                .item(El::new().s(Width::growable()))
                .item(crate::action_buttons::load_files_button_with_progress(
                    tracked_files.clone(),
                    ButtonVariant::Outline,
                    ButtonSize::Small,
                    Some(IconName::Folder),
                    file_dialog_visible,
                ))
                .item(El::new().s(Width::growable()))
                .item(crate::action_buttons::clear_all_files_button(
                    &tracked_files,
                    &selected_variables,
                )),
            Column::new()
                .s(Gap::new().y(SPACING_4))
                .s(Padding::new().top(SPACING_4).right(SPACING_4))
                .s(Height::fill())
                .s(Width::growable())
                .item(El::new().s(Height::fill()).s(Width::growable()).child(
                    Column::new().s(Width::fill()).s(Height::fill()).item(
                        El::new().s(Height::fill()).s(Width::fill()).child_signal({
                            let tracked_files_for_map = tracked_files.clone();
                            let selected_variables_for_map = selected_variables.clone();
                            file_count_broadcaster.signal().map(move |file_count| {
                                if file_count == 0 {
                                    empty_state_hint("Click 'Load Files' to add waveform files.")
                                        .unify()
                                } else {
                                    create_stable_tree_view(
                                        tracked_files_for_map.clone(),
                                        selected_variables_for_map.clone(),
                                        app_config.clone(),
                                    )
                                    .unify()
                                }
                            })
                        }),
                    ),
                )),
        ))
}

/// Create the main files panel with header and content
pub fn files_panel(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    load_files_button: impl Element + 'static,
    app_config: crate::config::AppConfig,
) -> impl Element {
    let file_count_broadcaster = tracked_files.files.signal_vec().len().broadcast();
    El::new()
        .s(Height::fill())
        .child(crate::panel_layout::create_panel(
            Row::new()
                .s(Gap::new().x(SPACING_8))
                .s(Align::new().center_y())
                .item(El::new().s(Font::new().no_wrap()).child("Files & Scopes"))
                .item(El::new().s(Width::growable()))
                .item(load_files_button)
                .item(El::new().s(Width::growable()))
                .item(crate::action_buttons::clear_all_files_button(
                    &tracked_files,
                    &selected_variables,
                )),
            Column::new()
                .s(Gap::new().y(SPACING_4))
                .s(Padding::new().top(SPACING_4).right(SPACING_4))
                .s(Height::fill())
                .s(Width::growable())
                .item(El::new().s(Height::fill()).s(Width::growable()).child(
                    Column::new().s(Width::fill()).s(Height::fill()).item(
                        El::new().s(Height::fill()).s(Width::fill()).child_signal({
                            let tracked_files_for_map = tracked_files.clone();
                            let selected_variables_for_map = selected_variables.clone();
                            file_count_broadcaster.signal().map(move |file_count| {
                                if file_count == 0 {
                                    empty_state_hint("Click 'Load Files' to add waveform files.")
                                        .unify()
                                } else {
                                    create_stable_tree_view(
                                        tracked_files_for_map.clone(),
                                        selected_variables_for_map.clone(),
                                        app_config.clone(),
                                    )
                                    .unify()
                                }
                            })
                        }),
                    ),
                )),
        ))
}

#[derive(Clone)]
struct FileSortKey {
    filename_key: String,
    prefix_key: String,
}

fn disambiguation_labels(files: &[TrackedFile]) -> HashMap<String, String> {
    if files.is_empty() {
        return HashMap::new();
    }

    let paths = files
        .iter()
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();

    generate_smart_labels(&paths)
}

fn compute_sort_metadata(
    files: &[TrackedFile],
    disambiguation_map: &HashMap<String, String>,
) -> HashMap<String, FileSortKey> {
    let mut filename_counts: HashMap<String, usize> = HashMap::with_capacity(files.len());
    for file in files {
        *filename_counts
            .entry(file.filename.to_lowercase())
            .or_insert(0) += 1;
    }

    let mut metadata = HashMap::with_capacity(files.len());
    for file in files {
        let filename_key = file.filename.to_lowercase();
        let duplicate_count = filename_counts.get(&filename_key).copied().unwrap_or(0);
        let prefix_key = if duplicate_count > 1 {
            disambiguation_map
                .get(&file.path)
                .and_then(|label| {
                    label
                        .rsplit_once('/')
                        .map(|(prefix, _)| prefix.to_lowercase())
                })
                .unwrap_or_else(|| {
                    std::path::Path::new(&file.path)
                        .parent()
                        .map(|parent| parent.to_string_lossy().to_lowercase())
                        .unwrap_or_default()
                })
        } else {
            String::new()
        };

        metadata.insert(
            file.path.clone(),
            FileSortKey {
                filename_key,
                prefix_key,
            },
        );
    }

    metadata
}

/// Create the stable tree view component for file display
pub fn create_stable_tree_view(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
) -> impl Element {
    El::new().s(Width::fill()).s(Height::fill()).child_signal({
        let tracked_files_for_closure = tracked_files.clone();
        let selected_variables_for_closure = selected_variables.clone();
        let app_config_for_closure = app_config.clone();

        tracked_files
            .files_vec_signal
            .signal_cloned()
            .map(move |files| {
                let disambiguation_map = Arc::new(disambiguation_labels(&files));
                let sort_metadata = compute_sort_metadata(&files, disambiguation_map.as_ref());

                let mut entries: Vec<_> = files
                    .into_iter()
                    .map(|file| {
                        let path = file.path.clone();
                        let key =
                            sort_metadata
                                .get(&path)
                                .cloned()
                                .unwrap_or_else(|| FileSortKey {
                                    filename_key: file.filename.to_lowercase(),
                                    prefix_key: String::new(),
                                });
                        (file, key)
                    })
                    .collect();

                entries.sort_by(|a, b| {
                    let a_key = &a.1;
                    let b_key = &b.1;
                    a_key
                        .filename_key
                        .cmp(&b_key.filename_key)
                        .then_with(|| a_key.prefix_key.cmp(&b_key.prefix_key))
                        .then_with(|| a.0.path.cmp(&b.0.path))
                });

                let disambiguation_map_for_labels = Arc::clone(&disambiguation_map);
                let tracked_files_for_items = tracked_files_for_closure.clone();
                let selected_variables_for_items = selected_variables_for_closure.clone();
                let app_config_for_items = app_config_for_closure.clone();

                Column::new()
                    .s(Width::fill())
                    .s(Height::fill())
                    .s(Gap::new().y(SPACING_2))
                    .update_raw_el(|raw_el| {
                        raw_el
                            .style("width", "100%")
                            .style("min-width", "fit-content")
                    })
                    .items(entries.into_iter().map(move |(tracked_file, _)| {
                        let map_for_iteration = Arc::clone(&disambiguation_map_for_labels);
                        let tracked_files_for_iteration = tracked_files_for_items.clone();
                        let selected_variables_for_iteration = selected_variables_for_items.clone();
                        let app_config_for_iteration = app_config_for_items.clone();
                        let smart_label =
                            compute_smart_label_for_file(&tracked_file, map_for_iteration.as_ref());
                        render_tracked_file_as_tree_item_with_label_and_expanded_state(
                            tracked_file,
                            smart_label,
                            tracked_files_for_iteration,
                            selected_variables_for_iteration,
                            app_config_for_iteration,
                        )
                    }))
                    .into_element()
            })
    })
}

/// Render a tracked file as a tree item with proper labeling and expanded state
pub fn render_tracked_file_as_tree_item_with_label_and_expanded_state(
    tracked_file: TrackedFile,
    smart_label: String,
    tracked_files_domain: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
) -> impl Element {
    let display_name = smart_label;
    let tree_data = match &tracked_file.state {
        shared::FileState::Loading(_) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::File)
                    .icon("circle-loader-2")
                    .disabled(true),
            ]
        }
        shared::FileState::Loaded(file_data) => {
            let children = file_data
                .scopes
                .iter()
                .map(|scope| convert_scope_to_tree_data(scope))
                .collect();
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::File)
                    .icon("file")
                    .on_remove(create_enhanced_file_remove_handler(
                        tracked_file.id.clone(),
                        tracked_files_domain.clone(),
                        selected_variables.clone(),
                    ))
                    .with_children(children),
            ]
        }
        shared::FileState::Failed(file_error) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("alert-circle")
                    .tooltip(format!(
                        "{}\nError: {}",
                        tracked_file.path,
                        file_error.user_friendly_message()
                    ))
                    .error_message(file_error.user_friendly_message())
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(
                        tracked_file.id.clone(),
                        tracked_files_domain.clone(),
                        selected_variables.clone(),
                    )),
            ]
        }
        shared::FileState::Missing(_file_path) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("file-x")
                    .tooltip(format!("{}\nFile no longer exists", tracked_file.path))
                    .error_message("File no longer exists".to_string())
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(
                        tracked_file.id.clone(),
                        tracked_files_domain.clone(),
                        selected_variables.clone(),
                    )),
            ]
        }
        shared::FileState::Unsupported(reason) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("circle-help")
                    .tooltip(format!("{}\nUnsupported: {}", tracked_file.path, reason))
                    .error_message(format!("Unsupported: {}", reason))
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(
                        tracked_file.id.clone(),
                        tracked_files_domain.clone(),
                        selected_variables.clone(),
                    )),
            ]
        }
    };

    tree_view()
        .data(tree_data)
        .size(TreeViewSize::Medium)
        .variant(TreeViewVariant::Basic)
        .show_icons(true)
        .show_checkboxes(true)
        .show_checkboxes_on_scopes_only(true)
        .single_scope_selection(true)
        .external_expanded(app_config.files_expanded_scopes.clone())
        .external_selected_vec(app_config.files_selected_scope.clone())
        .build()
}

/// Compute smart label for a single file with duplicate detection AND time intervals
pub fn compute_smart_label_for_file(
    target_file: &TrackedFile,
    disambiguation_map: &HashMap<String, String>,
) -> String {
    let base_name = disambiguation_map
        .get(&target_file.path)
        .cloned()
        .unwrap_or_else(|| target_file.filename.clone());

    match &target_file.state {
        shared::FileState::Loaded(waveform_file) => {
            if let (Some(min_ns), Some(max_ns)) =
                (waveform_file.min_time_ns, waveform_file.max_time_ns)
            {
                let min_seconds = min_ns as f64 / 1_000_000_000.0;
                let max_seconds = max_ns as f64 / 1_000_000_000.0;

                let time_range = if max_seconds < 1.0 {
                    format!("{:.0}–{:.0}ms", min_seconds * 1000.0, max_seconds * 1000.0)
                } else if max_seconds < 60.0 {
                    if max_seconds.fract() == 0.0 && min_seconds.fract() == 0.0 {
                        format!("{:.0}–{:.0}s", min_seconds, max_seconds)
                    } else {
                        format!("{:.1}–{:.1}s", min_seconds, max_seconds)
                    }
                } else {
                    let min_minutes = min_seconds / 60.0;
                    let max_minutes = max_seconds / 60.0;
                    format!("{:.1}–{:.1}min", min_minutes, max_minutes)
                };

                format!("{} ({})", base_name, time_range)
            } else {
                base_name
            }
        }
        shared::FileState::Loading(_) => {
            format!("{} (Loading...)", base_name)
        }
        _ => base_name,
    }
}

/// Render tracked file reactively with expanded scopes signal
pub fn render_tracked_file_reactive(
    tracked_file: TrackedFile,
    expanded_scopes_signal: impl zoon::Signal<Item = indexmap::IndexSet<String>>
    + 'static
    + std::marker::Unpin,
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
) -> impl Element {
    let all_files = tracked_files.files_vec_signal.get_cloned();
    let disambiguation_map = disambiguation_labels(&all_files);
    let smart_label = compute_smart_label_for_file(&tracked_file, &disambiguation_map);

    El::new().child_signal({
        let tracked_file = tracked_file.clone();
        let smart_label = smart_label.clone();
        let tracked_files = tracked_files.clone();
        let selected_variables = selected_variables.clone();
        let app_config_for_closure = app_config.clone();
        expanded_scopes_signal.map(move |_expanded_scopes| {
            render_tracked_file_as_tree_item_with_label_and_expanded_state(
                tracked_file.clone(),
                smart_label.clone(),
                tracked_files.clone(),
                selected_variables.clone(),
                app_config_for_closure.clone(),
            )
            .into_element()
        })
    })
}

/// Convert scope data to tree view item data
pub fn convert_scope_to_tree_data(scope: &ScopeData) -> TreeViewItemData {
    let mut children = Vec::new();

    let mut child_refs: Vec<&ScopeData> = scope.children.iter().collect();
    child_refs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    for child_scope in child_refs {
        children.push(convert_scope_to_tree_data(child_scope));
    }

    let scope_tree_id = format!("scope_{}", scope.id);

    TreeViewItemData::new(scope_tree_id, scope.name.clone())
        .item_type(TreeViewItemType::Folder)
        .with_children(children)
}

/// Create enhanced file remove handler with proper cleanup
pub fn create_enhanced_file_remove_handler(
    _file_id: String,
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Fn(&str) + 'static {
    move |id: &str| {
        let current_tracked_files = tracked_files.get_current_files();
        crate::file_operations::cleanup_file_related_state(
            id,
            &current_tracked_files,
            &selected_variables,
        );

        tracked_files.file_removed_relay.send(id.to_string());
    }
}

/// Create empty state hint for when no files are loaded
pub fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}

/// Files panel with dynamic height based on config
pub fn files_panel_with_height(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    El::new()
        .s(Height::exact_signal(
            crate::dragging::files_panel_height_signal(app_config.clone()).map(|h| h as u32),
        ))
        .s(Width::growable())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin").style_signal(
                "scrollbar-color",
                primary_6()
                    .map(|thumb| {
                        moonzoon_novyui::tokens::color::primary_3()
                            .map(move |track| format!("{} {}", thumb, track))
                    })
                    .flatten(),
            )
        })
        .child(files_panel(
            tracked_files.clone(),
            selected_variables.clone(),
            button().label("Load Files").disabled(true).build(), // Placeholder - no file_dialog_visible access
            app_config.clone(),
        ))
}
