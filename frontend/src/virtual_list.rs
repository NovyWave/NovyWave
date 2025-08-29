use zoon::*;
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::color::{neutral_2, neutral_8, neutral_11, primary_3, primary_6};
use wasm_bindgen::JsCast;

use crate::types::{VariableWithContext, filter_variables_with_context};

fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}

// ===== CORE VIRTUAL LIST FUNCTIONS =====
// 
// WARNING: Avoid excessive // zoon::println! logging in virtual lists and viewport handlers.
// Virtual lists trigger frequent resize/scroll events that can generate thousands of log
// entries per second, corrupting dev_server.log with multi-gigabyte binary data and
// making compilation errors impossible to see.
//
// SOLUTION: Use debug_throttled() for high-frequency events, debug_critical() for errors.
// This prevents log corruption while preserving essential debugging capability.
//

pub fn virtual_variables_list(variables: Vec<VariableWithContext>, search_filter: String) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // zoon::println!("VIRTUAL_VARIABLES_LIST called with {} variables, filter: '{}'", variables.len(), search_filter);
    // Handle special cases first (empty states)
    if variables.is_empty() && search_filter.starts_with("Select a scope") {
        return Column::new().item(empty_state_hint(&search_filter));
    }
    
    if variables.is_empty() {
        return Column::new().item(empty_state_hint("No variables in selected scope"));
    }
    
    // Apply search filter
    let filtered_variables = filter_variables_with_context(&variables, &search_filter);
    
    if filtered_variables.is_empty() {
        return Column::new().item(empty_state_hint("No variables match search filter"));
    }
    
    // FIXED-HEIGHT VIRTUAL LIST - only render ~15 visible items
    // PHASE 1 TEST: Use signal-based version with always(400.0)
    // SIMPLE FILL: Use Height::fill() directly (works now that Column hierarchy is fixed)
    rust_virtual_variables_list_simple_fill(filtered_variables)
}

pub fn rust_virtual_variables_list_simple_fill(variables: Vec<VariableWithContext>) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // zoon::println!("RUST_VIRTUAL_VARIABLES_LIST_SIMPLE_FILL called with {} variables", variables.len());
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
                    rust_virtual_variables_list_with_signal(variables, virtual_list_height)
                )
        )
}

// REMOVED: rust_virtual_variables_list - unused legacy function

pub fn rust_virtual_variables_list_with_signal(
    variables: Vec<VariableWithContext>,
    height_signal: Broadcaster<MutableSignal<u32>>
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // zoon::println!("RUST_VIRTUAL_VARIABLES_LIST_WITH_SIGNAL called with {} variables", variables.len());
    let total_items = variables.len();
    let item_height = 24.0;
    
    // Get initial visible count for scroll handler  
    let initial_visible_count = ((400.0_f64 / item_height).ceil() as usize + 5).min(total_items);
    
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
        (0..initial_pool_size).map(|_| {
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
        }).collect()
    );
    
    // ===== SCROLL VELOCITY TRACKING =====
    let last_scroll_time = Mutable::new(0.0);
    let last_scroll_position = Mutable::new(0.0);
    let scroll_velocity = Mutable::new(0.0); // pixels per second
    
    // ===== REACTIVE VISIBLE COUNT =====
    let visible_count = Mutable::new(initial_visible_count);
    
    // ===== HEIGHT SIGNAL LISTENER WITH POOL RESIZING =====
    Task::start({
        let height_signal = height_signal.clone();
        let visible_count = visible_count.clone();
        let element_pool = element_pool.clone();
        let scroll_velocity = scroll_velocity.clone();
        async move {
            height_signal.signal().for_each(|height| {
                let new_visible_count = ((height as f64 / item_height).ceil() as usize + 5).min(total_items);
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
                    let additional_elements: Vec<VirtualElementState> = (current_pool_size..needed_pool_size).map(|_| {
                        VirtualElementState {
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
                        }
                    }).collect();
                    
                    element_pool.lock_mut().extend(additional_elements);
                } else if needed_pool_size < current_pool_size {
                    // Shrink pool efficiently (but keep minimum buffer)
                    let min_pool_size = (new_visible_count + base_buffer).max(20);
                    element_pool.lock_mut().truncate(min_pool_size);
                }
                
                async {}
            }).await;
        }
    });
    
    // ===== UPDATE visible_end WHEN visible_count CHANGES =====
    Task::start({
        let visible_count = visible_count.clone();
        let visible_start = visible_start.clone();
        let visible_end = visible_end.clone();
        async move {
            visible_count.signal().for_each(move |new_count| {
                let current_start = visible_start.get();
                let new_end = (current_start + new_count).min(total_items);
                visible_end.set_neq(new_end);
                async {}
            }).await;
        }
    });
    
    // ===== POOL UPDATE TASK =====
    // Update pool elements when visible range changes
    Task::start({
        let variables = variables.clone();
        let element_pool = element_pool.clone();
        let visible_start = visible_start.clone();
        let visible_end = visible_end.clone();
        async move {
            map_ref! {
                let start = visible_start.signal(),
                let end = visible_end.signal() => (*start, *end)
            }.for_each_sync(move |(start, end)| {
                let pool = element_pool.lock_ref();
                let visible_count = end - start;
                
                // Update each pool element efficiently
                for (pool_index, element_state) in pool.iter().enumerate() {
                    let absolute_index = start + pool_index;
                    
                    if pool_index < visible_count && absolute_index < variables.len() {
                        // This element should be visible - update content
                        if let Some(variable_context) = variables.get(absolute_index) {
                            element_state.name_signal.set_neq(variable_context.signal.name.clone());
                            element_state.type_signal.set_neq(
                                format!("{} {}-bit", variable_context.signal.signal_type, variable_context.signal.width)
                            );
                            element_state.position_signal.set_neq(
                                (absolute_index as f64 * item_height) as i32
                            );
                            element_state.visible_signal.set_neq(true);
                            
                            // Set context data for click handlers
                            element_state.file_id_signal.set_neq(variable_context.file_id.clone());
                            element_state.scope_id_signal.set_neq(variable_context.scope_id.clone());
                            element_state.variable_signal.set_neq(Some(variable_context.signal.clone()));
                            
                            // Debug log removed
                            
                            // Set previous variable name for prefix highlighting
                            let previous_name = if absolute_index > 0 {
                                variables.get(absolute_index - 1).map(|prev_variable| prev_variable.signal.name.clone())
                            } else {
                                None
                            };
                            element_state.previous_name_signal.set_neq(previous_name);
                            
                            // Determine if this variable is selected
                            // We need to get the scope path from TRACKED_FILES
                            let is_selected = {
                                use crate::state::{TRACKED_FILES, is_variable_selected, find_scope_full_name};
                                let tracked_files = TRACKED_FILES.lock_ref();
                                if let Some(tracked_file) = tracked_files.iter().find(|f| f.id == variable_context.file_id) {
                                    if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
                                        if let Some(scope_path) = find_scope_full_name(&waveform_file.scopes, &variable_context.scope_id) {
                                            is_variable_selected(&tracked_file.path, &scope_path, &variable_context.signal.name)
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            };
                            element_state.is_selected_signal.set_neq(is_selected);
                            element_state.absolute_index_signal.set_neq(absolute_index);
                        }
                    } else {
                        // Hide this element
                        element_state.visible_signal.set_neq(false);
                        element_state.position_signal.set_neq(-9999);
                        // Clear context data when hidden
                        element_state.file_id_signal.set_neq(String::new());
                        element_state.scope_id_signal.set_neq(String::new());
                        element_state.variable_signal.set_neq(None);
                        element_state.absolute_index_signal.set_neq(0);
                    }
                }
            }).await;
        }
    });
    
    // ===== REACTIVE SELECTION STATE TASK =====
    // Update selection highlighting in real-time when SELECTED_VARIABLES changes
    Task::start({
        let variables = variables.clone();
        let element_pool = element_pool.clone();
        let visible_start = visible_start.clone();
        let visible_end = visible_end.clone();
        async move {
            use crate::state::{SELECTED_VARIABLES, TRACKED_FILES, is_variable_selected, find_scope_full_name};
            
            SELECTED_VARIABLES.signal_vec_cloned().for_each(move |_| {
                // zoon::println!("REACTIVE SELECTION STATE: SELECTED_VARIABLES changed, updating highlighting");
                let pool = element_pool.lock_ref();
                let start = visible_start.get();
                let end = visible_end.get();
                let visible_count = end - start;
                
                // Update selection state for all visible elements
                for (pool_index, element_state) in pool.iter().enumerate() {
                    let absolute_index = start + pool_index;
                    
                    if pool_index < visible_count && absolute_index < variables.len() {
                        if let Some(variable_context) = variables.get(absolute_index) {
                            // Re-evaluate selection state using current SELECTED_VARIABLES
                            let is_selected = {
                                let tracked_files = TRACKED_FILES.lock_ref();
                                if let Some(tracked_file) = tracked_files.iter().find(|f| f.id == variable_context.file_id) {
                                    if let shared::FileState::Loaded(waveform_file) = &tracked_file.state {
                                        if let Some(scope_path) = find_scope_full_name(&waveform_file.scopes, &variable_context.scope_id) {
                                            is_variable_selected(&tracked_file.path, &scope_path, &variable_context.signal.name)
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            };
                            
                            // Update the selection signal to trigger UI update
                            element_state.is_selected_signal.set_neq(is_selected);
                            // zoon::println!("Updated selection state for variable '{}': {}", variable_context.signal.name, is_selected);
                        }
                    }
                }
                async {}
            }).await;
        }
    });
    
    
    Column::new()
        .item(
            // ===== SCROLL CONTAINER WITH SIGNAL HEIGHT =====
            // CRITICAL: Uses Height::exact_signal() instead of Height::exact()
            El::new()
                .s(Width::fill())
                .s(Height::exact_signal(height_signal.signal()))  // 🔥 KEY CHANGE: Signal-based height
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
                                                let new_velocity = (position_delta / time_delta) * 1000.0; // px/second
                                                scroll_velocity.set_neq(new_velocity);
                                                
                                                // ===== DYNAMIC POOL ADJUSTMENT ON FAST SCROLL =====
                                                if new_velocity > 800.0 {
                                                    let current_pool_size = element_pool.lock_ref().len();
                                                    let needed_size = visible_count.get() + 15; // Fast scroll: larger buffer
                                                    
                                                    if needed_size > current_pool_size {
                                                        let additional_elements: Vec<VirtualElementState> = (current_pool_size..needed_size).map(|_| {
                                                            VirtualElementState {
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
                                                            }
                                                        }).collect();
                                                        
                                                        element_pool.lock_mut().extend(additional_elements);
                                                    }
                                                }
                                            }
                                        }
                                        
                                        last_scroll_time.set_neq(current_time);
                                        last_scroll_position.set_neq(new_scroll_top);
                                        scroll_top.set_neq(new_scroll_top);
                                        
                                        let start_index = (new_scroll_top / item_height).floor() as usize;
                                        let end_index = (start_index + visible_count.get()).min(total_items);
                                        
                                        visible_start.set_neq(start_index);
                                        visible_end.set_neq(end_index);
                                        
                                    }
                                }
                            }) as Box<dyn FnMut(_)>);
                            
                            html_el.add_event_listener_with_callback(
                                "scroll",
                                scroll_closure.as_ref().unchecked_ref()
                            ).unwrap();
                            
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
                                    if let Some(target_el) = event.current_target()
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
                            }) as Box<dyn FnMut(web_sys::MouseEvent)>);
                            
                            html_el.add_event_listener_with_callback(
                                "mousemove",
                                mousemove_closure.as_ref().unchecked_ref()
                            ).unwrap();
                            
                            // Clear hover when mouse leaves
                            let mouseleave_closure = Closure::wrap(Box::new({
                                let hovered_index = hovered_index.clone();
                                move |_event: web_sys::MouseEvent| {
                                    hovered_index.set_neq(None);
                                }
                            }) as Box<dyn FnMut(web_sys::MouseEvent)>);
                            
                            html_el.add_event_listener_with_callback(
                                "mouseleave",
                                mouseleave_closure.as_ref().unchecked_ref()
                            ).unwrap();
                            
                            mousemove_closure.forget();
                            mouseleave_closure.forget();
                        }
                        
                        raw_el.style("scrollbar-width", "thin")
                            .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
                    }
                })
                .child(
                    // ===== HYBRID STABLE POOL CONTAINER =====
                    // No more child_signal recreation - stable elements only!
                    Stack::new()
                        .s(Width::fill())
                        .s(Height::exact((total_items as f64 * item_height) as u32))
                        .layers_signal_vec(
                            element_pool.signal_vec_cloned().map({
                                let hovered_index = hovered_index.clone();
                                move |element_state| {
                                    create_stable_variable_element_hybrid(element_state, hovered_index.clone())
                                }
                            })
                        )
                )
        )
}

// REMOVED: rust_virtual_variables_list_dynamic_wrapper - unused legacy function

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
fn create_stable_variable_element_hybrid(state: VirtualElementState, hovered_index: Mutable<Option<usize>>) -> impl Element {
    // zoon::println!("CREATE_STABLE_VARIABLE_ELEMENT_HYBRID called");
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Transform::with_signal_self(
            map_ref! {
                let position = state.position_signal.signal(),
                let visible = state.visible_signal.signal() => {
                    if *visible {
                        Transform::new().move_down(*position)
                    } else {
                        Transform::new().move_down(-9999)  // Hide off-screen
                    }
                }
            }
        ))
        .s(Padding::new().x(12).y(2))
        .s(Background::new().color_signal(
            map_ref! {
                let is_selected = state.is_selected_signal.signal(),
                let hovered_idx = hovered_index.signal(),
                let absolute_idx = state.absolute_index_signal.signal(),
                let primary_bg = primary_3(),
                let hover_bg = neutral_4(),
                let default_bg = neutral_2() => {
                    if *is_selected {
                        *primary_bg
                    } else if hovered_idx.as_ref() == Some(absolute_idx) {
                        *hover_bg
                    } else {
                        *default_bg
                    }
                }
            }
        ))
        .update_raw_el(|raw_el| {
            raw_el.style("cursor", "pointer")
        })
        .on_click({
            let file_id_signal = state.file_id_signal.clone();
            let scope_id_signal = state.scope_id_signal.clone();
            let variable_signal = state.variable_signal.clone();
            move || {
                // zoon::println!("CLICK HANDLER TRIGGERED!");
                let file_id = file_id_signal.get_cloned();
                let scope_id = scope_id_signal.get_cloned();
                // zoon::println!("Click context: file_id={}, scope_id={}", file_id, scope_id);
                if let Some(variable) = variable_signal.get_cloned() {
                    // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
                    if let Some(selected_var) = crate::actors::create_selected_variable(variable, &file_id, &scope_id) {
                        let selected_variables = crate::actors::selected_variables_domain();
                        selected_variables.variable_clicked_relay.send(selected_var);
                    }
                } else {
                    // zoon::println!("No variable found in variable_signal");
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
            )
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color_signal(primary_6()).size(12))
                .s(Font::new().no_wrap())
                .child(Text::with_signal(state.type_signal.signal_cloned()))
        )
}


// REMOVED: virtual_variable_row - unused legacy function

// REMOVED: simple_variable_row - unused legacy function

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
    if let Some(last_boundary) = prefix_str.rfind('_')
        .or_else(|| prefix_str.rfind('.'))
        .or_else(|| prefix_str.rfind('['))
        .or_else(|| prefix_str.rfind('$')) {
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
                                        move || {
                                            // zoon::println!("PARAGRAPH CLICK HANDLER TRIGGERED!");
                                            let file_id = file_id_signal.get_cloned();
                                            let scope_id = scope_id_signal.get_cloned();
                                            // zoon::println!("Paragraph click context: file_id={}, scope_id={}", file_id, scope_id);
                                            if let Some(variable) = variable_signal.get_cloned() {
                                                // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
                                                if let Some(selected_var) = crate::actors::create_selected_variable(variable, &file_id, &scope_id) {
                                                    let selected_variables = crate::actors::selected_variables_domain();
                                                    selected_variables.variable_clicked_relay.send(selected_var);
                                                }
                                            } else {
                                                // zoon::println!("No variable found in paragraph variable_signal");
                                            }
                                        }
                                    })
                                    .into_element()
                            } else {
                                // No shared prefix, display normally with click handler
                                Paragraph::new()
                                    .s(Font::new().color_signal(neutral_11()))
                                    .content(name.clone())
                                    .on_click({
                                        let file_id_signal = file_id_signal.clone();
                                        let scope_id_signal = scope_id_signal.clone();
                                        let variable_signal = variable_signal.clone();
                                        move || {
                                            // zoon::println!("PARAGRAPH CLICK HANDLER TRIGGERED (no prefix)!");
                                            let file_id = file_id_signal.get_cloned();
                                            let scope_id = scope_id_signal.get_cloned();
                                            // zoon::println!("Paragraph click context: file_id={}, scope_id={}", file_id, scope_id);
                                            if let Some(variable) = variable_signal.get_cloned() {
                                                // zoon::println!("Paragraph variable found: {}", variable.name);
                                                use crate::state::add_selected_variable;
                                                add_selected_variable(variable, &file_id, &scope_id);
                                            } else {
                                                // zoon::println!("No variable found in paragraph variable_signal");
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
                                    move || {
                                        // zoon::println!("PARAGRAPH CLICK HANDLER TRIGGERED (first item)!");
                                        let file_id = file_id_signal.get_cloned();
                                        let scope_id = scope_id_signal.get_cloned();
                                        // zoon::println!("Paragraph click context: file_id={}, scope_id={}", file_id, scope_id);
                                        if let Some(variable) = variable_signal.get_cloned() {
                                            // zoon::println!("Paragraph variable found: {}", variable.name);
                                            use crate::state::add_selected_variable;
                                            add_selected_variable(variable, &file_id, &scope_id);
                                        } else {
                                            // zoon::println!("No variable found in paragraph variable_signal");
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

// REMOVED: simple_variables_list - unused legacy function

