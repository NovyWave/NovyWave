use moonzoon_novyui::tokens::color::{
    neutral_2, neutral_4, neutral_8, neutral_11, primary_3, primary_6,
};
use moonzoon_novyui::*;
use zoon::*;
use wasm_bindgen::JsCast;

use crate::selected_variables::VariableWithContext;

// Virtual list performance constants - extracted from hardcoded values
const FALLBACK_CONTAINER_HEIGHT: f64 = 400.0; // Typical panel height for initial calculations
const FAST_SCROLL_VELOCITY_THRESHOLD: f64 = 800.0; // Threshold for dynamic buffer adjustment
const FAST_SCROLL_BUFFER_SIZE: usize = 15; // Additional buffer elements for fast scrolling

fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(SPACING_20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}

// ===== CORE VIRTUAL LIST FUNCTIONS =====
//
// Virtual lists trigger frequent resize/scroll events that can generate thousands of log
// entries per second, corrupting dev_server.log with multi-gigabyte binary data and
// making compilation errors impossible to see.
//
// SOLUTION: Use debug_throttled() for high-frequency events, debug_critical() for errors.
// This prevents log corruption while preserving essential debugging capability.
//

/// ✅ PERFORMANCE: Pre-filtered virtual list - no filtering inside, just rendering
pub fn virtual_variables_list_pre_filtered(
    filtered_variables: Vec<VariableWithContext>,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // Handle empty states efficiently
    if filtered_variables.is_empty() {
        return Column::new().item(empty_state_hint("No variables match search filter"));
    }

    // Direct rendering - filtering already done at signal level
    rust_virtual_variables_list_simple_fill(filtered_variables, selected_variables)
}

pub fn rust_virtual_variables_list_simple_fill(
    variables: Vec<VariableWithContext>,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // DYNAMIC HEIGHT SOLUTION: Parent-child pattern with real viewport monitoring
    let height_mutable = Mutable::new(400u32); // Start with reasonable default
    let virtual_list_height = Broadcaster::new(height_mutable.signal());

    Column::new()
        .s(Height::fill()) // Parent fills available space
        .s(Width::fill())
        .item(
            El::new()
                .s(Width::fill())
                .s(Height::fill()) // Monitor parent claims all available height
                .on_viewport_size_change({
                    let height_mutable = height_mutable.clone();
                    move |_width, height| {
                        let constrained_height = (height as f64).max(100.0) as u32;
                        height_mutable.set_neq(constrained_height);
                    }
                })
                .child(
                    // Child uses exact height from parent measurement
                    rust_virtual_variables_list_with_signal(variables, virtual_list_height, selected_variables.clone()),
                ),
        )
}

pub fn rust_virtual_variables_list_with_signal(
    variables: Vec<VariableWithContext>,
    height_signal: Broadcaster<MutableSignal<u32>>,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    let total_items = variables.len();
    let item_height = 24.0;

    // Get initial visible count for scroll handler - use fallback height until container renders
    // Using fallback height for initial calculations before container renders
    let initial_container_height = FALLBACK_CONTAINER_HEIGHT; // Fallback: typical panel height
    let initial_visible_count =
        ((initial_container_height / item_height).ceil() as usize + 5).min(total_items);

    // ===== VIRTUAL SCROLLING STATE =====
    let scroll_top = Mutable::new(0.0);
    let visible_start = Mutable::new(0usize);
    let visible_end = Mutable::new(initial_visible_count.min(total_items));

    // ===== HYBRID STABLE POOL WITH DYNAMIC SIZING =====
    // Start with basic buffer, will adjust based on scroll velocity
    let base_buffer = 5; // Minimum buffer

    // Single source of truth for hover state - eliminates race conditions
    let hovered_index: Mutable<Option<usize>> = Mutable::new(None);
    let initial_pool_size = initial_visible_count + base_buffer;
    let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(
        (0..initial_pool_size)
            .map(|_| {
                VirtualElementState {
                    name_signal: Mutable::new(String::new()),
                    type_signal: Mutable::new(String::new()),
                    position_signal: Mutable::new(-9999), // Start hidden
                    visible_signal: Mutable::new(false),
                    previous_name_signal: Mutable::new(None),
                    file_id_signal: Mutable::new(String::new()),
                    scope_id_signal: Mutable::new(String::new()),
                    variable_signal: Mutable::new(None),
                    is_selected_signal: Mutable::new(false),
                    absolute_index_signal: Mutable::new(0), // Track which virtual item this element represents
                }
            })
            .collect(),
    );

    // ===== SCROLL VELOCITY TRACKING =====
    let last_scroll_time = Mutable::new(0.0);
    let last_scroll_position = Mutable::new(0.0);
    let scroll_velocity = Mutable::new(0.0); // pixels per second

    // ===== REACTIVE VISIBLE COUNT =====
    let visible_count = Mutable::new(initial_visible_count);

    // ===== COORDINATED TASK: Pool Management + Updates =====
    // OPTIMIZATION: Single task handles height changes, pool resizing, and batched DOM updates
    Task::start({
        let height_signal = height_signal.clone();
        let visible_count = visible_count.clone();
        let element_pool = element_pool.clone();
        let scroll_velocity = scroll_velocity.clone();
        let visible_start = visible_start.clone();
        let visible_end = visible_end.clone();
        async move {
            height_signal
                .signal()
                .for_each(move |height| {
                    let new_visible_count =
                        ((height as f64 / item_height).ceil() as usize + 5).min(total_items);
                    visible_count.set_neq(new_visible_count);

                    // ===== DYNAMIC POOL RESIZING BASED ON VELOCITY =====
                    // Calculate buffer based on current scroll velocity
                    let current_velocity = scroll_velocity.get();
                    let velocity_buffer = if current_velocity > 1000.0 {
                        15 // Fast scrolling: larger buffer
                    } else if current_velocity > 500.0 {
                        10 // Medium scrolling: medium buffer
                    } else {
                        base_buffer // Slow/no scrolling: minimal buffer (5)
                    };

                    let needed_pool_size = new_visible_count + velocity_buffer;
                    let current_pool_size = element_pool.lock_ref().len();

                    if needed_pool_size > current_pool_size {
                        // Grow pool efficiently with MutableVec
                        let additional_elements: Vec<VirtualElementState> = (current_pool_size
                            ..needed_pool_size)
                            .map(|_| VirtualElementState {
                                name_signal: Mutable::new(String::new()),
                                type_signal: Mutable::new(String::new()),
                                position_signal: Mutable::new(-9999),
                                visible_signal: Mutable::new(false),
                                previous_name_signal: Mutable::new(None),
                                file_id_signal: Mutable::new(String::new()),
                                scope_id_signal: Mutable::new(String::new()),
                                variable_signal: Mutable::new(None),
                                is_selected_signal: Mutable::new(false),
                                absolute_index_signal: Mutable::new(0),
                            })
                            .collect();

                        element_pool.lock_mut().extend(additional_elements);
                    } else if needed_pool_size < current_pool_size {
                        // Shrink pool efficiently (but keep minimum buffer)
                        let min_pool_size = (new_visible_count + base_buffer).max(20);
                        element_pool.lock_mut().truncate(min_pool_size);
                    }

                    // ✅ OPTIMIZATION: Update visible range immediately for better coordination
                    let current_start = visible_start.get();
                    let new_end = (current_start + new_visible_count).min(total_items);
                    visible_end.set_neq(new_end);

                    async {}
                })
                .await;
        }
    });

    // ===== OPTIMIZED POOL UPDATE TASK WITH DOM BATCHING =====
    // Update pool elements when visible range OR selection state changes
    Task::start({
        let variables = variables.clone();
        let element_pool = element_pool.clone();
        let visible_start = visible_start.clone();
        let visible_end = visible_end.clone();
        let selected_variables = selected_variables.clone();
        async move {
            map_ref! {
                let start = visible_start.signal(),
                let end = visible_end.signal(),
                let selected_vars = selected_variables.variables_vec_actor.signal() => (*start, *end, selected_vars.clone())
            }.for_each(move |(start, end, selected_vars)| {
                let element_pool = element_pool.clone();
                let variables = variables.clone();
                async move {
                    // ✅ OPTIMIZATION: Batch DOM updates using requestAnimationFrame-like pattern
                    // Process updates in single batch to optimize Chrome event loop
                    struct BatchedUpdate {
                        element_index: usize,
                        name: String,
                        type_str: String,
                        position: i32,
                        visible: bool,
                        file_id: String,
                        scope_id: String,
                        variable: Option<shared::Signal>,
                        previous_name: Option<String>,
                        is_selected: bool,
                        absolute_index: usize,
                    }

                    let pool = element_pool.lock_ref();
                    let visible_count = end - start;
                    let mut batched_updates = Vec::with_capacity(pool.len());

                    // ✅ OPTIMIZATION: Pre-compute selection lookup to reduce clone operations
                    // use crate::state::{find_scope_full_name}; // Unused

                    let tracked_files: Vec<shared::TrackedFile> = vec![];

                    // ✅ REACTIVE: Build selection index from current selected variables
                    let selected_vars_index: std::collections::HashSet<String> = selected_vars.iter()
                        .map(|var| var.unique_id.clone())
                        .collect();


                    // Collect all updates first (reduce lock contention)
                    for (pool_index, _element_state) in pool.iter().enumerate() {
                        let absolute_index = start + pool_index;

                        if pool_index < visible_count && absolute_index < variables.len() {
                            // This element should be visible - prepare update
                            if let Some(variable_context) = variables.get(absolute_index) {
                                // ✅ MEMORY OPTIMIZATION: Use references where possible to reduce clones
                                let name = &variable_context.signal.name;
                                let signal_type = &variable_context.signal.signal_type;
                                let width = variable_context.signal.width;
                                let file_id = &variable_context.file_id;
                                let scope_id = &variable_context.scope_id;

                                // Format type string once
                                let type_str = format!("{} {}-bit", signal_type, width);

                                // Set previous variable name for prefix highlighting
                                let previous_name = if absolute_index > 0 {
                                    variables.get(absolute_index - 1).map(|prev_variable| prev_variable.signal.name.clone())
                                } else {
                                    None
                                };

                                // Determine if this variable is selected (Actor+Relay compatible)
                                let is_selected = if let Some(tracked_file) = tracked_files.iter().find(|f| &f.id == file_id) {
                                    if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {

                                        // Try to find scope by direct name match first
                                        let scope_path = waveform_file.scopes.iter()
                                            .find(|scope| scope.name == *scope_id)
                                            .map(|scope| scope.name.clone())
                                            .or_else(|| {
                                                // If scope_id has a suffix like ".s", try without it
                                                if let Some(dot_pos) = scope_id.rfind('.') {
                                                    let scope_without_suffix = &scope_id[..dot_pos];
                                                    waveform_file.scopes.iter()
                                                        .find(|scope| scope.name == scope_without_suffix)
                                                        .map(|scope| scope.name.clone())
                                                } else {
                                                    None
                                                }
                                            });

                                        if let Some(_found_scope_path) = scope_path {
                                            // Use original scope_id to maintain format consistency with selected variables
                                            let unique_id = format!("{}|{}|{}", tracked_file.path, scope_id, name);
                                            let selected = selected_vars_index.contains(&unique_id);
                                            selected
                                        } else { false }
                                    } else { false }
                                } else { false };

                                batched_updates.push(BatchedUpdate {
                                    element_index: pool_index,
                                    name: name.clone(), // Only clone when needed for update
                                    type_str,
                                    position: (absolute_index as f64 * item_height) as i32,
                                    visible: true,
                                    file_id: file_id.clone(),
                                    scope_id: scope_id.clone(),
                                    variable: Some(variable_context.signal.clone()),
                                    previous_name,
                                    is_selected,
                                    absolute_index,
                                });
                            }
                        } else {
                            // Hide this element
                            batched_updates.push(BatchedUpdate {
                                element_index: pool_index,
                                name: String::new(),
                                type_str: String::new(),
                                position: -9999,
                                visible: false,
                                file_id: String::new(),
                                scope_id: String::new(),
                                variable: None,
                                previous_name: None,
                                is_selected: false,
                                absolute_index: 0,
                            });
                        }
                    }

                    // No need to drop - tracked_files is now a Vec, not a lock reference

                    // ✅ OPTIMIZATION: Apply all batched updates in single pass
                    // This reduces DOM update frequency and improves Chrome performance
                    for update in batched_updates {
                        if let Some(element_state) = pool.get(update.element_index) {
                            // Apply all updates atomically
                            element_state.name_signal.set_neq(update.name);
                            element_state.type_signal.set_neq(update.type_str);
                            element_state.position_signal.set_neq(update.position);
                            element_state.visible_signal.set_neq(update.visible);
                            element_state.file_id_signal.set_neq(update.file_id);
                            element_state.scope_id_signal.set_neq(update.scope_id);
                            element_state.variable_signal.set_neq(update.variable);
                            element_state.previous_name_signal.set_neq(update.previous_name);
                            element_state.is_selected_signal.set_neq(update.is_selected);
                            element_state.absolute_index_signal.set_neq(update.absolute_index);
                        }
                    }
                }
            }).await;
        }
    });

    // Selection state updates are now handled in the optimized pool update task above

    Column::new().item(
        // ===== SCROLL CONTAINER WITH SIGNAL HEIGHT =====
        // CRITICAL: Uses Height::exact_signal() instead of Height::exact()
        El::new()
            .s(Width::fill())
            .s(Height::exact_signal(height_signal.signal())) // 🔥 KEY CHANGE: Signal-based height
            .s(Background::new().color_signal(neutral_2()))
            .s(Padding::new().top(4))
            .update_raw_el({
                let scroll_top = scroll_top.clone();
                let visible_start = visible_start.clone();
                let visible_end = visible_end.clone();
                let visible_count = visible_count.clone();
                let last_scroll_time = last_scroll_time.clone();
                let last_scroll_position = last_scroll_position.clone();
                let scroll_velocity = scroll_velocity.clone();
                let element_pool = element_pool.clone();

                move |el| {
                    if let Some(html_el) = el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                        html_el.set_id("virtual-container-signal");
                        html_el.style().set_property("overflow-y", "auto").unwrap();
                        html_el.style().set_property("display", "block").unwrap();

                        let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                            let scroll_top = scroll_top.clone();
                            let visible_start = visible_start.clone();
                            let visible_end = visible_end.clone();
                            let visible_count = visible_count.clone();
                            let last_scroll_time = last_scroll_time.clone();
                            let last_scroll_position = last_scroll_position.clone();
                            let scroll_velocity = scroll_velocity.clone();
                            let element_pool = element_pool.clone();

                            move |_event: web_sys::Event| {
                                if let Some(scroll_el) = web_sys::window()
                                    .and_then(|w| w.document())
                                    .and_then(|d| d.get_element_by_id("virtual-container-signal"))
                                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                                {
                                    let new_scroll_top = scroll_el.scroll_top() as f64;

                                    // ===== VELOCITY CALCULATION =====
                                    let current_time = web_sys::window()
                                        .and_then(|w| Some(w.performance()?.now()))
                                        .unwrap_or(0.0);

                                    let last_time = last_scroll_time.get();
                                    let last_position = last_scroll_position.get();

                                    if last_time > 0.0 {
                                        let time_delta = current_time - last_time;
                                        let position_delta = (new_scroll_top - last_position).abs();

                                        if time_delta > 0.0 {
                                            let new_velocity =
                                                (position_delta / time_delta) * 1000.0; // px/second
                                            scroll_velocity.set_neq(new_velocity);

                                            // ===== DYNAMIC POOL ADJUSTMENT ON FAST SCROLL =====
                                            if new_velocity > FAST_SCROLL_VELOCITY_THRESHOLD {
                                                let current_pool_size =
                                                    element_pool.lock_ref().len();
                                                let needed_size =
                                                    visible_count.get() + FAST_SCROLL_BUFFER_SIZE; // Fast scroll: larger buffer

                                                if needed_size > current_pool_size {
                                                    let additional_elements: Vec<
                                                        VirtualElementState,
                                                    > =
                                                        (current_pool_size..needed_size)
                                                            .map(|_| VirtualElementState {
                                                                name_signal: Mutable::new(
                                                                    String::new(),
                                                                ),
                                                                type_signal: Mutable::new(
                                                                    String::new(),
                                                                ),
                                                                position_signal: Mutable::new(
                                                                    -9999,
                                                                ),
                                                                visible_signal: Mutable::new(false),
                                                                previous_name_signal: Mutable::new(
                                                                    None,
                                                                ),
                                                                file_id_signal: Mutable::new(
                                                                    String::new(),
                                                                ),
                                                                scope_id_signal: Mutable::new(
                                                                    String::new(),
                                                                ),
                                                                variable_signal: Mutable::new(None),
                                                                is_selected_signal: Mutable::new(
                                                                    false,
                                                                ),
                                                                absolute_index_signal: Mutable::new(
                                                                    0,
                                                                ),
                                                            })
                                                            .collect();

                                                    element_pool
                                                        .lock_mut()
                                                        .extend(additional_elements);
                                                }
                                            }
                                        }
                                    }

                                    last_scroll_time.set_neq(current_time);
                                    last_scroll_position.set_neq(new_scroll_top);
                                    scroll_top.set_neq(new_scroll_top);

                                    let start_index =
                                        (new_scroll_top / item_height).floor() as usize;
                                    let end_index =
                                        (start_index + visible_count.get()).min(total_items);

                                    visible_start.set_neq(start_index);
                                    visible_end.set_neq(end_index);
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
                    }

                    el
                }
            })
            .update_raw_el({
                let hovered_index = hovered_index.clone();
                let scroll_top = scroll_top.clone();
                move |raw_el| {
                    // Add mouse tracking for centralized hover - FIXED coordinate system
                    if let Some(html_el) = raw_el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                        let mousemove_closure = Closure::wrap(Box::new({
                            let hovered_index = hovered_index.clone();
                            let scroll_top = scroll_top.clone();
                            move |event: web_sys::MouseEvent| {
                                // FIX: Use clientY relative to viewport, then get container bounds
                                if let Some(target_el) = event
                                    .current_target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
                                {
                                    let container_rect = target_el.get_bounding_client_rect();
                                    let client_y = event.client_y() as f64;

                                    // Calculate mouse Y relative to container top
                                    let container_relative_y = client_y - container_rect.top();

                                    // Add scroll offset to get absolute position within virtual content
                                    let current_scroll = scroll_top.get();
                                    let absolute_y = container_relative_y + current_scroll;
                                    let item_index = (absolute_y / item_height).floor() as usize;

                                    if item_index < total_items && container_relative_y >= 0.0 {
                                        hovered_index.set_neq(Some(item_index));
                                    } else {
                                        hovered_index.set_neq(None);
                                    }
                                }
                            }
                        })
                            as Box<dyn FnMut(web_sys::MouseEvent)>);

                        html_el
                            .add_event_listener_with_callback(
                                "mousemove",
                                mousemove_closure.as_ref().unchecked_ref(),
                            )
                            .unwrap();

                        // Clear hover when mouse leaves
                        let mouseleave_closure = Closure::wrap(Box::new({
                            let hovered_index = hovered_index.clone();
                            move |_event: web_sys::MouseEvent| {
                                hovered_index.set_neq(None);
                            }
                        })
                            as Box<dyn FnMut(web_sys::MouseEvent)>);

                        html_el
                            .add_event_listener_with_callback(
                                "mouseleave",
                                mouseleave_closure.as_ref().unchecked_ref(),
                            )
                            .unwrap();

                        mousemove_closure.forget();
                        mouseleave_closure.forget();
                    }

                    raw_el.style("scrollbar-width", "thin").style_signal(
                        "scrollbar-color",
                        primary_6()
                            .map(|thumb| {
                                primary_3().map(move |track| format!("{} {}", thumb, track))
                            })
                            .flatten(),
                    )
                }
            })
            .child(
                // ===== HYBRID STABLE POOL CONTAINER =====
                // No more child_signal recreation - stable elements only!
                Stack::new()
                    .s(Width::fill())
                    .s(Height::exact((total_items as f64 * item_height) as u32))
                    .layers_signal_vec(element_pool.signal_vec_cloned().map({
                        let hovered_index = hovered_index.clone();
                        let selected_variables_for_map = selected_variables.clone();
                        move |element_state| {
                            create_stable_variable_element_hybrid(
                                element_state,
                                hovered_index.clone(),
                                selected_variables_for_map.clone(),
                            )
                        }
                    })),
            ),
    )
}

// ===== ROW RENDERING FUNCTIONS =====

// ===== STABLE ELEMENT POOL VIRTUALIZATION =====

#[derive(Clone)]
struct VirtualElementState {
    name_signal: Mutable<String>,
    type_signal: Mutable<String>,
    position_signal: Mutable<i32>,
    visible_signal: Mutable<bool>,
    previous_name_signal: Mutable<Option<String>>,
    // Context data for click handlers
    file_id_signal: Mutable<String>,
    scope_id_signal: Mutable<String>,
    variable_signal: Mutable<Option<shared::Signal>>,
    // Selection state for visual feedback
    is_selected_signal: Mutable<bool>,
    // Track which virtual item this element represents
    absolute_index_signal: Mutable<usize>,
}

// ===== HYBRID STABLE ELEMENT =====
// Optimized version for the hybrid MutableVec approach
fn create_stable_variable_element_hybrid(
    state: VirtualElementState,
    hovered_index: Mutable<Option<usize>>,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Element {
    // Clone selected_variables for the multiple closures that need it
    let selected_variables_1 = selected_variables.clone();
    let selected_variables_2 = selected_variables.clone(); 
    let selected_variables_3 = selected_variables.clone();
    
    Row::new()
        .s(Gap::new().x(SPACING_8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Transform::with_signal_self(map_ref! {
            let position = state.position_signal.signal(),
            let visible = state.visible_signal.signal() => {
                if *visible {
                    Transform::new().move_down(*position)
                } else {
                    Transform::new().move_down(-9999)  // Hide off-screen
                }
            }
        }))
        .s(Padding::new().x(SPACING_12).y(SPACING_2))
        .s(Background::new().color_signal(map_ref! {
            let is_selected = state.is_selected_signal.signal(),
            let hovered_idx = hovered_index.signal(),
            let absolute_idx = state.absolute_index_signal.signal(),
            let primary_bg = primary_3(),
            let hover_bg = neutral_4(),
            let default_bg = neutral_2() => {
                let color = if *is_selected {
                    *primary_bg
                } else if hovered_idx.as_ref() == Some(absolute_idx) {
                    *hover_bg
                } else {
                    *default_bg
                };
                color
            }
        }))
        .update_raw_el(|raw_el| raw_el.style("cursor", "pointer"))
        .on_click({
            let file_id_signal = state.file_id_signal.clone();
            let scope_id_signal = state.scope_id_signal.clone();
            let variable_signal = state.variable_signal.clone();
            let selected_variables_for_click = selected_variables_1.clone();
            move || {
                let file_id = file_id_signal.get_cloned();
                let scope_id = scope_id_signal.get_cloned();
                if let Some(variable) = variable_signal.get_cloned() {
                    // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
                    if let Some(selected_var) = crate::selected_variables::create_selected_variable(
                        variable, &file_id, &scope_id,
                    ) {
                        let selected_variables = &selected_variables_for_click;
                        selected_variables
                            .variable_clicked_relay
                            .send(selected_var.unique_id.clone());
                    } else {
                    }
                } else {
                }
            }
        })
        .item(
            // Variable name with prefix highlighting
            create_variable_name_display(
                state.name_signal.signal_cloned(),
                state.previous_name_signal.signal_cloned(),
                state.file_id_signal.clone(),
                state.scope_id_signal.clone(),
                state.variable_signal.clone(),
                selected_variables_2.clone(),
            ),
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color_signal(primary_6()).size(12))
                .s(Font::new().no_wrap())
                .child(Text::with_signal(state.type_signal.signal_cloned())),
        )
}

// ===== SHARED PREFIX HIGHLIGHTING FUNCTIONS =====

/// Detect shared prefix between two variable names using word boundary logic
/// Returns (prefix_length, has_meaningful_prefix)
fn detect_shared_prefix(current: &str, previous: &str) -> (usize, bool) {
    if current.is_empty() || previous.is_empty() {
        return (0, false);
    }

    // Find common prefix character by character
    let mut prefix_len = 0;
    let current_chars: Vec<char> = current.chars().collect();
    let previous_chars: Vec<char> = previous.chars().collect();

    for (i, (c1, c2)) in current_chars.iter().zip(previous_chars.iter()).enumerate() {
        if c1 == c2 {
            prefix_len = i + 1;
        } else {
            break;
        }
    }

    // Apply minimum threshold and word boundary logic
    if prefix_len < 3 {
        return (0, false);
    }

    // Look for word boundary within the common prefix
    let prefix_str: String = current_chars.iter().take(prefix_len).collect();
    if let Some(last_boundary) = prefix_str
        .rfind('_')
        .or_else(|| prefix_str.rfind('.'))
        .or_else(|| prefix_str.rfind('['))
        .or_else(|| prefix_str.rfind('$'))
    {
        let boundary_prefix_len = last_boundary + 1; // Include the boundary character
        if boundary_prefix_len >= 3 {
            return (boundary_prefix_len, true);
        }
    }

    // If no good word boundary, use character-level prefix if it's long enough
    if prefix_len >= 5 {
        return (prefix_len, true);
    }

    (0, false)
}

/// Create variable name display with prefix highlighting
fn create_variable_name_display(
    name_signal: impl zoon::Signal<Item = String> + Unpin + 'static,
    previous_name_signal: impl zoon::Signal<Item = Option<String>> + Unpin + 'static,
    file_id_signal: Mutable<String>,
    scope_id_signal: Mutable<String>,
    variable_signal: Mutable<Option<shared::Signal>>,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Element {
    El::new()
        .s(Font::new().size(14).no_wrap())
        .child_signal(
            map_ref! {
                let name = name_signal,
                let previous_name = previous_name_signal => {
                    match previous_name {
                        Some(prev) => {
                            let (prefix_len, has_prefix) = detect_shared_prefix(name, prev);

                            if has_prefix && prefix_len > 0 {
                                let prefix = &name[..prefix_len];
                                let suffix = &name[prefix_len..];

                                // Use Paragraph for inline text coloring with click handler
                                Paragraph::new()
                                    .content(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8())) // Dimmed prefix
                                            .child(prefix)
                                    )
                                    .content(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_11())) // Normal suffix
                                            .child(suffix)
                                    )
                                    .on_click({
                                        let file_id_signal = file_id_signal.clone();
                                        let scope_id_signal = scope_id_signal.clone();
                                        let variable_signal = variable_signal.clone();
                                        let selected_variables_clone = selected_variables.clone();
                                        move || {
                                            let file_id = file_id_signal.get_cloned();
                                            let scope_id = scope_id_signal.get_cloned();
                                            if let Some(variable) = variable_signal.get_cloned() {
                                                // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
                                                if let Some(selected_var) = crate::selected_variables::create_selected_variable(variable, &file_id, &scope_id) {
                                                    selected_variables_clone.variable_clicked_relay.send(selected_var.unique_id.clone());
                                                }
                                            } else {
                                            }
                                        }
                                    })
                                    .into_element()
                            } else {
                                // No shared prefix, display normally with click handler
                                Paragraph::new()
                                    .s(Font::new().color_signal(neutral_11())) // Normal text color
                                    .content(name.clone())
                                    .on_click({
                                        let file_id_signal = file_id_signal.clone();
                                        let scope_id_signal = scope_id_signal.clone();
                                        let variable_signal = variable_signal.clone();
                                        let selected_variables_clone = selected_variables.clone();
                                        move || {
                                            let file_id = file_id_signal.get_cloned();
                                            let scope_id = scope_id_signal.get_cloned();
                                            if let Some(variable) = variable_signal.get_cloned() {
                                                // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
                                                if let Some(selected_var) = crate::selected_variables::create_selected_variable(variable, &file_id, &scope_id) {
                                                    selected_variables_clone.variable_clicked_relay.send(selected_var.unique_id.clone());
                                                }
                                            } else {
                                            }
                                        }
                                    })
                                    .into_element()
                            }
                        },
                        None => {
                            // First item, no previous to compare with - add click handler
                            Paragraph::new()
                                .s(Font::new().color_signal(neutral_11()))
                                .content(name.clone())
                                .on_click({
                                    let file_id_signal = file_id_signal.clone();
                                    let scope_id_signal = scope_id_signal.clone();
                                    let variable_signal = variable_signal.clone();
                                    let selected_variables_clone = selected_variables.clone();
                                    move || {
                                        let file_id = file_id_signal.get_cloned();
                                        let scope_id = scope_id_signal.get_cloned();
                                        if let Some(variable) = variable_signal.get_cloned() {
                                            // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
                                            if let Some(selected_var) = crate::selected_variables::create_selected_variable(variable, &file_id, &scope_id) {
                                                selected_variables_clone.variable_clicked_relay.send(selected_var.unique_id);
                                            }
                                        } else {
                                        }
                                    }
                                })
                                .into_element()
                        }
                    }
                }
            }
        )
}

// ===== SUPPORT FUNCTIONS =====
