use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_4, neutral_8, neutral_11, neutral_12, primary_3, primary_6};
use moonzoon_novyui::tokens::theme::Theme;
use moonzoon_novyui::*;
use zoon::events::{Click, KeyDown};
use zoon::*;
use zoon::map_ref;
use crate::dataflow::{atom::Atom, relay, Actor};
use std::collections::{HashMap, HashSet};
use indexmap::IndexSet;
use shared::{UpMsg, FileSystemItem};
use futures::{stream::StreamExt as FuturesStreamExt, stream::FusedStream};

/// TreeView sync actors to handle bi-directional sync between FilePickerDomain and TreeView
/// Replaces zoon::Task pattern with proper Actor+Relay architecture
struct TreeViewSyncActors {
    domain_to_treeview_sync: Actor<()>,
    treeview_to_domain_sync: Actor<()>,
}

impl TreeViewSyncActors {
    fn new(
        domain: crate::config::FilePickerDomain,
        external_expanded: zoon::Mutable<IndexSet<String>>,
    ) -> Self {
        // ✅ ACTOR+RELAY: Domain → TreeView sync
        let domain_to_treeview_sync = Actor::new((), {
            let external_expanded_sync = external_expanded.clone();
            let domain_clone = domain.clone();
            async move |_state| {
                zoon::println!("📂 SYNC_ACTOR: Starting domain → TreeView sync");
                let mut signal_stream = domain_clone.expanded_directories_signal().to_stream();
                while let Some(index_set) = signal_stream.next().await {
                    zoon::println!("📂 SYNC_ACTOR: Syncing {} expanded dirs from domain to TreeView", index_set.len());
                    external_expanded_sync.set_neq(index_set);
                }
            }
        });

        // ✅ ACTOR+RELAY: TreeView → Domain sync
        let treeview_to_domain_sync = Actor::new((), {
            let domain_for_expansion = domain.clone();
            let external_expanded_for_expansion = external_expanded.clone();
            async move |_state| {
                let mut previous_expanded: IndexSet<String> = IndexSet::new();
                let mut external_signal_stream = external_expanded_for_expansion.signal_cloned().to_stream();
                let mut is_first_sync = true;
                zoon::println!("📂 EXPANSION_HANDLER: Starting TreeView → Domain sync");

                while let Some(current_expanded) = external_signal_stream.next().await {
                    zoon::println!("📂 TREEVIEW_EXPAND: External expanded changed, now has {} directories", current_expanded.len());

                    // Skip the first sync if it's empty to prevent clearing config on initialization
                    if is_first_sync && current_expanded.is_empty() {
                        zoon::println!("📂 TREEVIEW_EXPAND: Skipping initial empty state to preserve config");
                        is_first_sync = false;
                        continue;
                    }
                    is_first_sync = false;

                    for path in current_expanded.iter() {
                        if !previous_expanded.contains(path) {
                            zoon::println!("📂 TREEVIEW_EXPAND: Expanding directory: {}", path);
                            domain_for_expansion.directory_expanded_relay.send(path.clone());
                            domain_for_expansion.directory_load_requested_relay.send(path.clone());
                        }
                    }

                    for path in previous_expanded.iter() {
                        if !current_expanded.contains(path) {
                            domain_for_expansion.directory_collapsed_relay.send(path.clone());
                        }
                    }

                    previous_expanded = current_expanded;
                }
            }
        });

        Self {
            domain_to_treeview_sync,
            treeview_to_domain_sync,
        }
    }
}


/// Initialize directories on first use and when restored from config
fn initialize_directories_and_request_contents(
    file_picker_domain: &crate::config::FilePickerDomain,
) -> crate::dataflow::Actor<()> {
    file_picker_domain.directory_load_requested_relay.send("/".to_string());

    let domain_clone = file_picker_domain.clone();

    let initialization_actor = crate::dataflow::Actor::new((), async move |state_handle| {

        let mut current_expanded = domain_clone.expanded_directories_actor.signal().to_stream();
        if let Some(expanded) = current_expanded.next().await {
            if expanded.is_empty() {
                if let Some(home_dir) = std::env::var("HOME").ok().or_else(|| std::env::var("USERPROFILE").ok()) {
                    domain_clone.directory_expanded_relay.send("/".to_string());
                    domain_clone.directory_expanded_relay.send(home_dir.clone());
                    domain_clone.directory_load_requested_relay.send(home_dir);
                } else {
                    domain_clone.directory_expanded_relay.send("/".to_string());
                }
            } else {
                for directory in &expanded {
                    domain_clone.directory_load_requested_relay.send(directory.clone());
                }
            }
        }

        domain_clone.directory_load_requested_relay.send("/".to_string());
        state_handle.set(());
    });

    initialization_actor
}


/// Build tree data for TreeView component using Actor signals
fn build_tree_data(
    root_path: &str,
    cache: &HashMap<String, Vec<FileSystemItem>>,
    errors: &HashMap<String, String>
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
                        .is_waveform_file(true)
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
    errors: &HashMap<String, String>
) -> TreeViewItemData {
    let mut item = TreeViewItemData::new(path.to_string(), name.to_string())
        .icon("folder".to_string())
        .item_type(TreeViewItemType::Folder);

    if let Some(_error) = errors.get(path) {
        // Directory has access error
        item = item.with_children(vec![
            TreeViewItemData::new("error".to_string(), "Can't access this directory".to_string())
                .disabled(true)
                .item_type(TreeViewItemType::Default)
        ]);
    } else if cache.contains_key(path) {
        let child_items = build_tree_data(path, cache, errors);

        if child_items.is_empty() {
            // Directory is empty
            item = item.with_children(vec![
                TreeViewItemData::new("empty".to_string(), "Empty".to_string())
                    .disabled(true)
                    .item_type(TreeViewItemType::Default)
            ]);
        } else {
            item = item.with_children(child_items);
        }
    } else {
        item = item.with_children(vec![
            TreeViewItemData::new("loading".to_string(), "Loading...".to_string())
                .disabled(true)
                .item_type(TreeViewItemType::Default)
        ]);
    }

    item
}

/// Main file paths dialog for selecting waveform files
pub fn file_paths_dialog(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
    file_dialog_visible: Atom<bool>,
    connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    let file_count_broadcaster = tracked_files.files.signal_vec().len().broadcast();

    // Simple selected files state - no bi-directional sync
    let selected_files = Atom::new(Vec::<String>::new());

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
                                .child(file_picker_content(&app_config, &selected_files, connection))
                        )
                        .item(
                            El::new()
                                .s(Padding::all(4))
                                .child_signal({
                                    let selected_files_for_map = selected_files.clone();
                                    selected_files.signal().map(move |selected_paths| {
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
                                                    El::new()
                                                        .update_raw_el({
                                                            let path_for_tooltip = path.clone();
                                                            move |raw_el| {
                                                                raw_el.attr("title", &path_for_tooltip)
                                                            }
                                                        })
                                                        .child(
                                                            badge(filename)
                                                                .variant(BadgeVariant::Outline)
                                                                .size(BadgeSize::Small)
                                                                .removable()
                                                                .on_remove({
                                                                    let path = path.clone();
                                                                    let selected_files_for_remove = selected_files_for_map.clone();
                                                                    move || {
                                                                        let mut current = selected_files_for_remove.get_cloned();
                                                                        current.retain(|p| p != &path);
                                                                        selected_files_for_remove.set(current);
                                                                    }
                                                                })
                                                                .build()
                                                        )
                                                }))
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
                                        .label_signal(
                                            map_ref! {
                                                let file_count = file_count_broadcaster.signal(),
                                                let selected_files = selected_files.signal() =>
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
                                                let selected_files = selected_files.signal() => {
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
    selected_files: &Atom<Vec<String>>,
    connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    // Use TreeView-compatible MutableVec that syncs one-way from Atom
    let selected_files_vec = zoon::MutableVec::<String>::new();

    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        // Scroll position handled by FilePickerDomain actors
        .update_raw_el(|raw_el| {
            raw_el
                .style("min-height", "0")      // Allow flex shrinking
        })
        .child(file_picker_tree(&app_config, selected_files_vec.clone(), connection.clone()))
}

/// File picker tree using FilePickerDomain Actors
pub fn file_picker_tree(
    app_config: &crate::config::AppConfig,
    selected_files: zoon::MutableVec<String>,
    connection: crate::connection::ConnectionAdapter,
) -> impl Element {
    // Initialize directories and request contents for expanded directories from config
    let initialization_actor = initialize_directories_and_request_contents(&app_config.file_picker_domain);

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
                .style_signal("scrollbar-color", primary_6().map(|thumb|
                    primary_3().map(move |track| format!("{} {}", thumb, track))
                ).flatten())
        })
        .child_signal({
            let initialization_actor_for_closure = initialization_actor.clone();
            cache_signal.map(move |cache| {
                zoon::println!("🌳 TREE_VIEW_SIGNAL: Cache signal fired! Cache contains {} keys", cache.len());
                zoon::println!("🌳 TREE_VIEW_SIGNAL: Cache keys: {:?}", cache.keys().collect::<Vec<_>>());
                zoon::println!("🌳 TREE_VIEW_SIGNAL: Checking if cache contains root key '/'...");
                if cache.contains_key("/") {
                    zoon::println!("✅ TREE_VIEW_SIGNAL: Root directory '/' found in cache! Rendering TreeView");
                    if let Some(root_items) = cache.get("/") {
                        zoon::println!("📊 TREE_VIEW_SIGNAL: Root directory contains {} items", root_items.len());
                    }
                    // Root directory loaded - show tree
                    let tree_data = build_tree_data("/", &cache, &std::collections::HashMap::new());

                    {
                        // Create Mutable and sync actors outside of TreeView to control lifecycle
                        use indexmap::IndexSet;
                        // ✅ FIX: Initialize external_expanded as empty - Domain→TreeView sync will populate it
                        let external_expanded = zoon::Mutable::new(IndexSet::<String>::new());
                        zoon::println!("📂 EXTERNAL_EXPANDED: Created empty external_expanded Mutable for TreeView sync");

                        // ✅ ACTOR+RELAY FIX: Replace zoon::Task with proper Actors
                        let sync_actors = TreeViewSyncActors::new(
                            domain_for_treeview.clone(),
                            external_expanded.clone(),
                        );

                        // Store initialization actor for proper lifecycle management
                        let _initialization_actor = initialization_actor_for_closure.clone();

                        El::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .after_remove(move |_| {
                                // ✅ PROPER ACTOR STORAGE: Keep all actors alive until element drops
                                drop(sync_actors);
                                drop(_initialization_actor);
                                zoon::println!("📂 ACTORS: TreeView and initialization actors properly dropped");
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
                                    .into_raw()
                            )
                            .into_element()
                    }
                } else {
                    // Still loading root directory
                    zoon::println!("⏳ TREE_VIEW_SIGNAL: Root directory '/' NOT found in cache. Showing 'Loading directory contents...'");
                    zoon::println!("🔍 TREE_VIEW_SIGNAL: Cache keys available: {:?}", cache.keys().collect::<Vec<_>>());
                    El::new()
                        .s(Padding::all(20))
                        .s(Font::new().color_signal(neutral_8()).italic())
                        .child("Loading directory contents...")
                        .into_element()
                }
            })
        })
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

/// Build hierarchical tree structure from cache data with expansion handlers
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
                        let _children = build_hierarchical_tree(&item.path, tree_cache, error_cache);
                        let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                            .icon("folder".to_string())
                            .item_type(TreeViewItemType::Folder)
                            .with_children(_children);

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