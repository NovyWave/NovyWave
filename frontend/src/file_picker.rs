use crate::dataflow::{Actor, atom::Atom};
use crate::file_operations::extract_filename;
use futures::StreamExt;
use indexmap::IndexSet;
use moonzoon_novyui::tokens::color::{
    neutral_1, neutral_2, neutral_4, neutral_8, neutral_11, neutral_12, primary_3, primary_6,
};
use moonzoon_novyui::tokens::theme::Theme;
use moonzoon_novyui::*;
use shared::FileSystemItem;
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use zoon::events::{Click, KeyDown};
use zoon::map_ref;
use zoon::*;

/// TreeView sync actors to handle bi-directional sync between FilePickerDomain and TreeView
/// Replaces zoon::Task pattern with proper Actor+Relay architecture
pub struct TreeViewSyncActors {
    // Keep actors alive by storing them in the struct
    _domain_to_treeview_sync: Actor<()>,
    _treeview_to_domain_sync: Actor<()>,
}

/// Selected files sync actors to handle bi-directional sync between FilePickerDomain and TreeView
/// Similar to expanded directories sync but for file selections
pub struct SelectedFilesSyncActors {
    _domain_to_treeview_sync: Actor<()>,
    _treeview_to_domain_sync: Actor<()>,
}

impl TreeViewSyncActors {
    pub fn new(
        domain: crate::config::FilePickerDomain,
        external_expanded: zoon::Mutable<IndexSet<String>>,
    ) -> Self {
        // Domain → TreeView sync with immediate initial sync
        let domain_to_treeview_sync = Actor::new((), {
            let external_expanded_sync = external_expanded.clone();
            let domain_clone = domain.clone();
            async move |_state| {
                // Immediately sync the current value on initialization
                let initial_value = domain_clone
                    .expanded_directories_signal()
                    .to_stream()
                    .next()
                    .await;
                if let Some(index_set) = initial_value {
                    external_expanded_sync.set_neq(index_set);
                }

                // Continue syncing future changes
                let mut signal_stream = domain_clone.expanded_directories_signal().to_stream();
                while let Some(index_set) = signal_stream.next().await {
                    external_expanded_sync.set_neq(index_set);
                }
            }
        });

        // TreeView → Domain sync
        let treeview_to_domain_sync = Actor::new((), {
            let domain_for_expansion = domain.clone();
            let external_expanded_for_expansion = external_expanded.clone();
            async move |_state| {
                let mut previous_expanded: IndexSet<String> = IndexSet::new();
                let mut external_signal_stream =
                    external_expanded_for_expansion.signal_cloned().to_stream();
                let mut is_first_sync = true;

                while let Some(current_expanded) = external_signal_stream.next().await {
                    crate::app::emit_trace(
                        "treeview_sync_external",
                        format!("incoming={current_expanded:?} previous={previous_expanded:?}"),
                    );
                    // Don't send events on first sync to avoid clearing config
                    if is_first_sync {
                        is_first_sync = false;
                        // Don't send events on first sync to avoid clearing config
                        // Just update previous_expanded to track state
                        previous_expanded = current_expanded;
                        continue;
                    }

                    for path in current_expanded.iter() {
                        if !previous_expanded.contains(path) {
                            crate::app::emit_trace(
                                "workspace_picker_expand_request",
                                format!("path={path}"),
                            );
                            domain_for_expansion
                                .directory_expanded_relay
                                .send(path.clone());
                            domain_for_expansion
                                .directory_load_requested_relay
                                .send(path.clone());
                        }
                    }

                    for path in previous_expanded.iter() {
                        if !current_expanded.contains(path) {
                            crate::app::emit_trace(
                                "workspace_picker_collapse_request",
                                format!("path={path}"),
                            );
                            domain_for_expansion
                                .directory_collapsed_relay
                                .send(path.clone());
                        }
                    }

                    previous_expanded = current_expanded;
                }
            }
        });

        Self {
            _domain_to_treeview_sync: domain_to_treeview_sync,
            _treeview_to_domain_sync: treeview_to_domain_sync,
        }
    }
}

impl SelectedFilesSyncActors {
    pub fn new(
        domain: crate::config::FilePickerDomain,
        selected_files_mutable: zoon::MutableVec<String>,
    ) -> Self {
        // Domain → TreeView sync using differential updates
        let domain_to_treeview_sync = Actor::new((), {
            let selected_files_sync = selected_files_mutable.clone();
            let domain_clone = domain.clone();
            async move |_state| {
                let mut signal_stream = domain_clone
                    .selected_files_vec_signal
                    .signal_cloned()
                    .to_stream();
                while let Some(files_vec) = signal_stream.next().await {
                    selected_files_sync.lock_mut().replace_cloned(files_vec);
                }
            }
        });

        // TreeView → Domain sync with manual diff detection
        let treeview_to_domain_sync = Actor::new((), {
            let domain_for_selection = domain.clone();
            let selected_files_for_selection = selected_files_mutable.clone();
            async move |_state| {
                let mut previous_files: Vec<String> = Vec::new();
                let mut mutable_signal_stream = selected_files_for_selection
                    .signal_vec_cloned()
                    .to_signal_cloned()
                    .to_stream();
                let mut is_first_sync = true;

                while let Some(current_files) = mutable_signal_stream.next().await {
                    // Skip the first sync if it's empty to prevent clearing selection on initialization
                    if is_first_sync && current_files.is_empty() {
                        is_first_sync = false;
                        continue;
                    }
                    is_first_sync = false;

                    // Send file selected events for new files
                    for file_path in current_files.iter() {
                        if !previous_files.contains(file_path) {
                            domain_for_selection
                                .file_selected_relay
                                .send(file_path.clone());
                        }
                    }

                    // Send file deselected events for removed files
                    for file_path in previous_files.iter() {
                        if !current_files.contains(file_path) {
                            domain_for_selection
                                .file_deselected_relay
                                .send(file_path.clone());
                        }
                    }

                    previous_files = current_files;
                }
            }
        });

        Self {
            _domain_to_treeview_sync: domain_to_treeview_sync,
            _treeview_to_domain_sync: treeview_to_domain_sync,
        }
    }
}

/// Initialize directories on first use and when restored from config
pub fn initialize_directories_and_request_contents(
    file_picker_domain: &crate::config::FilePickerDomain,
) -> crate::dataflow::Actor<()> {
    file_picker_domain
        .directory_load_requested_relay
        .send("/".to_string());

    let domain_clone = file_picker_domain.clone();

    let initialization_actor = crate::dataflow::Actor::new((), async move |state_handle| {
        let mut current_expanded = domain_clone.expanded_directories_actor.signal().to_stream();
        if let Some(expanded) = current_expanded.next().await {
            if expanded.is_empty() {
                if let Some(home_dir) = std::env::var("HOME")
                    .ok()
                    .or_else(|| std::env::var("USERPROFILE").ok())
                {
                    domain_clone.directory_expanded_relay.send("/".to_string());
                    domain_clone.directory_expanded_relay.send(home_dir.clone());
                    domain_clone.directory_load_requested_relay.send(home_dir);
                } else {
                    domain_clone.directory_expanded_relay.send("/".to_string());
                }
            } else {
                for directory in &expanded {
                    domain_clone
                        .directory_load_requested_relay
                        .send(directory.clone());
                }
            }
        }

        domain_clone
            .directory_load_requested_relay
            .send("/".to_string());
        state_handle.set(());
    });

    initialization_actor
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
    file_dialog_visible: Atom<bool>,
    connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    // Count files that are actually in loading state, not total files
    let loading_count_broadcaster = tracked_files
        .files
        .signal_vec()
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
                                        tracked_files_for_enter.clone(),
                                        selected_files_value,
                                        file_dialog_visible_for_enter.clone()
                                    );
                                    // Clear the selection after loading files so dialog is ready for next selection
                                    file_picker_domain_for_enter.clear_selection_relay.send(());
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
                                .child(file_picker_content(&app_config, connection))
                        )
                        .item(
                            El::new()
                                .s(Padding::all(4))
                                .child_signal({
                                    let file_picker_domain_for_len = file_picker_domain.clone();
                                    let file_picker_domain_for_tags = file_picker_domain.clone();
                                    file_picker_domain_for_len.selected_files.signal_vec().len().map(move |selected_count| {
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
                                                                                        file_picker_domain_for_tag.file_deselected_relay.send(path.clone());
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
                                                let selected_count = file_picker_domain_for_button.selected_files.signal_vec().len() =>
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
                                                let selected_count = file_picker_domain_for_disabled.selected_files.signal_vec().len() => {
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
                                                    tracked_files_for_press.clone(),
                                                    selected_files_value,
                                                    file_dialog_visible_for_press.clone()
                                                );
                                                // Clear the selection after loading files so dialog is ready for next selection
                                                file_picker_domain_for_press.clear_selection_relay.send(());
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
    // Use TreeView-compatible MutableVec that syncs one-way from Atom
    let selected_files_vec = zoon::MutableVec::<String>::new();

    let file_picker_domain = app_config.file_picker_domain.clone();

    // Create sync actors to connect TreeView MutableVec ↔ FilePickerDomain ActorVec
    let selected_files_sync =
        SelectedFilesSyncActors::new(file_picker_domain.clone(), selected_files_vec.clone());

    let _tree_rendering_stream = file_picker_domain.tree_rendering_relay.subscribe();

    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .after_remove(move |_| {
            // Keep sync actors alive until element is removed from DOM
            drop(selected_files_sync);
        })
        // Scroll position restoration with tree rendering coordination
        .viewport_y_signal({
            let scroll_position_actor = file_picker_domain.scroll_position_actor.clone();

            // Use map_ref! for simpler signal coordination without complex async block
            zoon::map_ref! {
                let position = scroll_position_actor.signal() => {
                    *position
                }
            }
        })
        // Scroll position saving with raw DOM event handling
        .update_raw_el({
            let scroll_relay = file_picker_domain.scroll_position_changed_relay.clone();
            move |raw_el| {
                // Use the same pattern as virtual_list.rs for raw DOM scroll events
                let html_el = raw_el.dom_element();
                let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                    move |_event: web_sys::Event| {
                        if let Some(target) = _event.current_target() {
                            if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                                let scroll_top = element.scroll_top();
                                scroll_relay.send(scroll_top);
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
                .style_signal(
                    "scrollbar-color",
                    primary_6()
                        .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
                        .flatten(),
                )
        })
        .child(file_picker_tree(
            &app_config,
            selected_files_vec.clone(),
            connection,
        ))
}

/// File picker tree using FilePickerDomain Actors
pub fn file_picker_tree(
    app_config: &crate::config::AppConfig,
    selected_files: zoon::MutableVec<String>,
    _connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    // Initialize directories and request contents for expanded directories from config
    let initialization_actor =
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
                .style_signal(
                    "scrollbar-color",
                    primary_6()
                        .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
                        .flatten(),
                )
        })
        .child_signal({
            let initialization_actor_for_closure = initialization_actor.clone();
            cache_signal.map(move |cache| {
                if cache.contains_key("/") {
                    // Root directory loaded - show tree
                    let tree_data = build_tree_data("/", &cache, &std::collections::HashMap::new());

                    {
                        // Create Mutable and sync actors outside of TreeView to control lifecycle
                        use indexmap::IndexSet;
                        // Initialize external_expanded as empty - sync will populate immediately
                        let external_expanded = zoon::Mutable::new(IndexSet::<String>::new());

                        // Create sync actors for bi-directional synchronization
                        let sync_actors = TreeViewSyncActors::new(
                            domain_for_treeview.clone(),
                            external_expanded.clone(),
                        );

                        // Selected files sync is already created in file_picker_content
                        // Don't create duplicate sync actors

                        // Store initialization actor for proper lifecycle management
                        let _initialization_actor = initialization_actor_for_closure.clone();

                        El::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .after_remove(move |_| {
                                // Keep all actors alive until element drops
                                drop(sync_actors);
                                drop(_initialization_actor);
                            })
                            // Signal tree rendering completion for scroll position coordination
                            .after_insert({
                                let tree_rendering_relay =
                                    domain_for_treeview.tree_rendering_relay.clone();
                                let scroll_position_actor =
                                    domain_for_treeview.scroll_position_actor.clone();
                                move |_element| {
                                    tree_rendering_relay.send(());

                                    // CRITICAL: Trigger scroll restoration after tree is rendered
                                    let position_actor = scroll_position_actor.clone();
                                    zoon::Task::start(async move {
                                        // Wait for DOM to fully settle
                                        zoon::Task::next_macro_tick().await;

                                        // Get current scroll position and trigger restore
                                        let position = position_actor
                                            .signal()
                                            .to_stream()
                                            .next()
                                            .await
                                            .unwrap_or(0);

                                        // Set scroll position via DOM manipulation
                                        if position > 0 {
                                            if let Some(window) = web_sys::window() {
                                                if let Some(document) = window.document() {
                                                    // Use querySelector to find the scroll container
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
                                    .external_selected_vec(selected_files.clone())
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
