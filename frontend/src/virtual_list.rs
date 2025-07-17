use zoon::*;
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::color::{neutral_2, neutral_8, neutral_11, primary_3, primary_6};
use wasm_bindgen::JsCast;
use once_cell::sync::Lazy;

use shared::{Signal, filter_variables};

fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}

// ===== CORE VIRTUAL LIST FUNCTIONS =====
// 
// WARNING: Avoid excessive zoon::println! logging in virtual lists and viewport handlers.
// Virtual lists trigger frequent resize/scroll events that can generate thousands of log
// entries per second, corrupting dev_server.log with multi-gigabyte binary data and
// making compilation errors impossible to see. Use logging sparingly and only for
// critical debugging that can be quickly disabled.
//

pub fn virtual_variables_list(variables: Vec<Signal>, search_filter: String) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // Handle special cases first (empty states)
    if variables.is_empty() && search_filter.starts_with("Select a scope") {
        return Column::new().item(empty_state_hint(&search_filter));
    }
    
    if variables.is_empty() {
        return Column::new().item(empty_state_hint("No variables in selected scope"));
    }
    
    // Apply search filter
    let filtered_variables = filter_variables(&variables, &search_filter);
    
    if filtered_variables.is_empty() {
        return Column::new().item(empty_state_hint("No variables match search filter"));
    }
    
    // FIXED-HEIGHT VIRTUAL LIST - only render ~15 visible items
    // PHASE 1 TEST: Use signal-based version with always(400.0)
    // SIMPLE FILL: Use Height::fill() directly (works now that Column hierarchy is fixed)
    rust_virtual_variables_list_simple_fill(filtered_variables)
}

pub fn rust_virtual_variables_list_simple_fill(variables: Vec<Signal>) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
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
                        // Remove height cap to allow unlimited panel height (Step 1)
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

#[allow(dead_code)]
pub fn rust_virtual_variables_list(variables: Vec<Signal>) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    let total_items = variables.len();
    let item_height = 24.0; // Fixed height per item - DO NOT CHANGE
    
    // ===== HEIGHT MANAGEMENT =====
    // CURRENT: Fixed 400px height (WORKING)
    // DYNAMIC: Should be updated by viewport monitoring
    // TODO: Add dynamic container height calculation
    
    // ===== VISIBLE ITEM CALCULATIONS =====
    // Calculate how many items fit in the container + buffer
    // Buffer of +5 items ensures smooth scrolling
    let initial_visible_count = ((400.0_f64 / item_height).ceil() as usize + 5).min(total_items);
    
    // ===== VIRTUAL SCROLLING STATE =====
    // These track the current scroll position and visible range
    let scroll_top = Mutable::new(0.0);        // Current scroll offset in pixels
    let visible_start = Mutable::new(0usize);  // First visible item index
    let visible_end = Mutable::new(initial_visible_count.min(total_items)); // Last visible item index
    
    // ===== DYNAMIC HEIGHT INFRASTRUCTURE (DISABLED) =====
    // ISSUE: DOM elements report clientHeight=0 during initialization
    // SOLUTION: Use ResizeObserver or defer height queries until after render
    // When fixed, implement reactive container height → visible_count updates
    
    
    Column::new()
        .item(
            // ===== SCROLL CONTAINER =====
            // This El creates the scrollable area with fixed dimensions
            // CRITICAL: Height::exact(400) creates proper clientHeight for scrolling
            // PROBLEM: Height::fill() results in clientHeight=0, breaking scroll
            El::new()
                .s(Width::fill())
                .s(Height::exact(400))  // FIXED HEIGHT - WORKING
                // .s(Height::fill())   // DYNAMIC HEIGHT - BREAKS SCROLLING
                .s(Background::new().color_signal(neutral_2()))
                .s(Padding::new().top(4))
                // ===== VIEWPORT SIZE MONITORING (DISABLED) =====
                // This would track container size changes for dynamic height
                // PROBLEM: Works for height detection but breaks scrolling when combined with Height::fill()
                /*
                .on_viewport_size_change({
                    let container_height = container_height.clone();
                    move |_width, height| {
                        // Use reasonable height constraints to prevent viewport size bugs
                        let actual_height = (height as f64).max(100.0).min(800.0); // Reasonable bounds
                        container_height.set_neq(actual_height);
                    }
                })
                */
                // ===== DOM MANIPULATION & SCROLL SETUP =====
                // This sets up the actual scrollable DOM element
                .update_raw_el({
                    let scroll_top = scroll_top.clone();
                    let visible_start = visible_start.clone();
                    let visible_end = visible_end.clone();
                    
                    move |el| {
                        // ===== SCROLL CONTAINER SETUP =====
                        // Configure the DOM element for scrolling
                        if let Some(html_el) = el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                            html_el.set_id("virtual-container"); // Unique ID for scroll event targeting
                            html_el.style().set_property("overflow-y", "auto").unwrap(); // Enable vertical scrolling
                            html_el.style().set_property("display", "block").unwrap(); // Block layout for proper sizing
                            
                            // ===== CRITICAL DIAGNOSTIC =====
                            // These values show the core problem: clientHeight=0 when using Height::fill()
                            
                            // ===== SCROLL EVENT HANDLER =====
                            // This handles scroll events and updates the visible range
                            let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                                let scroll_top = scroll_top.clone();
                                let visible_start = visible_start.clone();
                                let visible_end = visible_end.clone();
                                
                                move |_event: web_sys::Event| {
                                    // Find the scroll container element
                                    if let Some(scroll_el) = web_sys::window()
                                        .and_then(|w| w.document())
                                        .and_then(|d| d.get_element_by_id("virtual-container"))
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) 
                                    {
                                        // Get current scroll position
                                        let new_scroll_top = scroll_el.scroll_top() as f64;
                                        scroll_top.set_neq(new_scroll_top);
                                        
                                        // ===== VIRTUAL RANGE CALCULATION =====
                                        // Calculate which items should be visible based on scroll position
                                        let start_index = (new_scroll_top / item_height).floor() as usize;
                                        // CURRENT: Uses fixed initial_visible_count (WORKING)
                                        // DYNAMIC: Should use visible_count.get() for reactive height
                                        let end_index = (start_index + initial_visible_count).min(total_items);
                                        // let end_index = (start_index + visible_count.get()).min(total_items); // FOR DYNAMIC
                                        
                                        // Update the visible range state
                                        visible_start.set_neq(start_index);
                                        visible_end.set_neq(end_index);
                                        
                                    }
                                }
                            }) as Box<dyn FnMut(_)>);
                            
                            // ===== SCROLL EVENT REGISTRATION =====
                            // Attach the scroll handler to the DOM element
                            html_el.add_event_listener_with_callback(
                                "scroll",
                                scroll_closure.as_ref().unchecked_ref()
                            ).unwrap();
                            
                            // Prevent closure from being garbage collected
                            scroll_closure.forget();
                            
                            // ===== POST-SETUP DIAGNOSTIC =====
                            // Check if the container has proper scroll dimensions
                            // WORKING: clientHeight=400, scrollHeight=large_number
                            // BROKEN: clientHeight=0, scrollHeight=0 (when using Height::fill())
                            
                        }
                        
                        el
                    }
                })
                .update_raw_el(|raw_el| {
                    raw_el.style("scrollbar-width", "thin")
                        .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
                })
                .child(
                    // ===== VIRTUAL CONTENT AREA =====
                    // This El represents the total scrollable content height
                    // Its height = total_items * item_height, creating the scroll thumb size
                    El::new()
                        .s(Width::fill())
                        .s(Height::exact((total_items as f64 * item_height) as u32)) // Total virtual height
                        .child_signal(
                            // ===== REACTIVE CONTENT RENDERING =====
                            // This signal updates whenever the visible range changes
                            map_ref! {
                                let start = visible_start.signal(),
                                let end = visible_end.signal() => {
                                    // Optional: Debug visible range changes
                                    
                                    // ===== STACK + TRANSFORM PATTERN =====
                                    // Uses Stack with Transform positioning (from working backup)
                                    // This pattern ensures proper layered rendering
                                    Stack::new()
                                        .s(Width::fill())
                                        .s(Height::exact((total_items as f64 * item_height) as u32)) // Match parent height
                                        .layers(
                                            // ===== VISIBLE ITEM RENDERING =====
                                            // Only render items in the visible range [start..end]
                                            variables[*start..*end].iter().enumerate().map(|(i, signal)| {
                                                // Calculate absolute position in the full list
                                                let absolute_index = *start + i;
                                                // Get previous variable name for prefix highlighting
                                                let previous_name = if absolute_index > 0 {
                                                    variables.get(absolute_index - 1).map(|prev| prev.name.clone())
                                                } else {
                                                    None
                                                };
                                                // Position each item using Transform (absolute positioning)
                                                virtual_variable_row_positioned(signal.clone(), absolute_index as f64 * item_height, previous_name)
                                            })
                                        )
                                        .into_element() // Convert to unified Element type
                                }
                            }
                        )
                )
        )
}

pub fn rust_virtual_variables_list_with_signal(
    variables: Vec<Signal>,
    height_signal: Broadcaster<MutableSignal<u32>>
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
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
    let initial_pool_size = initial_visible_count + base_buffer;
    let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(
        (0..initial_pool_size).map(|_| {
            VirtualElementState {
                name_signal: Mutable::new(String::new()),
                type_signal: Mutable::new(String::new()),
                position_signal: Mutable::new(-9999), // Start hidden
                visible_signal: Mutable::new(false),
                previous_name_signal: Mutable::new(None),
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
                        if let Some(signal) = variables.get(absolute_index) {
                            element_state.name_signal.set_neq(signal.name.clone());
                            element_state.type_signal.set_neq(
                                format!("{} {}-bit", signal.signal_type, signal.width)
                            );
                            element_state.position_signal.set_neq(
                                (absolute_index as f64 * item_height) as i32
                            );
                            element_state.visible_signal.set_neq(true);
                            
                            // Set previous variable name for prefix highlighting
                            let previous_name = if absolute_index > 0 {
                                variables.get(absolute_index - 1).map(|prev_signal| prev_signal.name.clone())
                            } else {
                                None
                            };
                            element_state.previous_name_signal.set_neq(previous_name);
                        }
                    } else {
                        // Hide this element
                        element_state.visible_signal.set_neq(false);
                        element_state.position_signal.set_neq(-9999);
                    }
                }
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
                .update_raw_el(|raw_el| {
                    raw_el.style("scrollbar-width", "thin")
                        .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
                })
                .child(
                    // ===== HYBRID STABLE POOL CONTAINER =====
                    // No more child_signal recreation - stable elements only!
                    Stack::new()
                        .s(Width::fill())
                        .s(Height::exact((total_items as f64 * item_height) as u32))
                        .layers_signal_vec(
                            element_pool.signal_vec_cloned().map(move |element_state| {
                                create_stable_variable_element_hybrid(element_state)
                            })
                        )
                )
        )
}

#[allow(dead_code)]
pub fn rust_virtual_variables_list_dynamic_wrapper(
    variables: Vec<Signal>
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // ===== SIGNAL BRIDGE =====
    // This Broadcaster allows parent to control child height
    let height_mutable = Mutable::new(400u32);
    let virtual_list_height = Broadcaster::new(height_mutable.signal());
    
    // ===== TEST: ADD HEIGHT::FILL() TO COLUMN =====
    // The Column itself needs Height::fill() to claim parent space!
    Column::new()
        .s(Height::fill())           // 🔥 ADD THIS TO COLUMN!
        .item(
            El::new()
                .s(Width::fill())
                .s(Height::fill())
                .on_viewport_size_change({
                    let height_mutable = height_mutable.clone();
                    move |_width, height| {
                        let constrained_height = (height as f64).max(100.0).min(800.0) as u32;
                        height_mutable.set_neq(constrained_height);
                    }
                })
                .child(
                    rust_virtual_variables_list_with_signal(variables, virtual_list_height)
                )
        )
}

// ===== ROW RENDERING FUNCTIONS =====

// OPTIMIZED VIRTUAL ROW: Uses Text::with_signal for reactive content updates
pub fn virtual_variable_row_optimized(signal: Signal, top_offset: f64) -> impl Element {
    // Create reactive text signals
    let name_signal = Mutable::new(signal.name.clone());
    let type_signal = Mutable::new(format!("{} {}-bit", signal.signal_type, signal.width));
    
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Transform::new().move_down(top_offset as i32))
        .s(Padding::new().x(12).y(2))
        .s(Background::new().color_signal(neutral_2()))
        .item(
            El::new()
                .s(Font::new().color_signal(neutral_11()).size(14))
                .s(Font::new().no_wrap())
                .child(Text::with_signal(name_signal.signal_cloned()))
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color_signal(primary_6()).size(12))
                .s(Font::new().no_wrap())
                .child(Text::with_signal(type_signal.signal_cloned()))
        )
}

// ===== STABLE ELEMENT POOL VIRTUALIZATION =====

#[derive(Clone)]
struct VirtualElementState {
    name_signal: Mutable<String>,
    type_signal: Mutable<String>,
    position_signal: Mutable<i32>,
    visible_signal: Mutable<bool>,
    previous_name_signal: Mutable<Option<String>>,
}

static VIRTUAL_ELEMENT_POOL: Lazy<MutableVec<VirtualElementState>> = lazy::default();



pub fn create_stable_virtual_list(
    variables: Vec<Signal>,
    visible_start: Mutable<usize>,
    visible_end: Mutable<usize>,
    item_height: f64,
    total_items: usize
) -> impl Element {
    
    // Calculate pool size (visible items + buffer)
    let pool_size = ((400.0 / item_height).ceil() as usize + 10).min(total_items);
    
    // Initialize element pool if empty
    if VIRTUAL_ELEMENT_POOL.lock_ref().is_empty() {
        let pool_elements: Vec<VirtualElementState> = (0..pool_size).map(|_| {
            VirtualElementState {
                name_signal: Mutable::new(String::new()),
                type_signal: Mutable::new(String::new()),
                position_signal: Mutable::new(-9999), // Start hidden
                visible_signal: Mutable::new(false),
                previous_name_signal: Mutable::new(None),
            }
        }).collect();
        
        VIRTUAL_ELEMENT_POOL.lock_mut().replace_cloned(pool_elements);
    }
    
    // Start the pool update task
    start_pool_update_task(variables, visible_start, visible_end, item_height);
    
    // Create the container with stable elements using Stack for absolute positioning
    Stack::new()
        .s(Width::fill())
        .s(Height::exact((total_items as f64 * item_height) as u32))
        .layers(
            // Create stable DOM elements that NEVER get recreated
            VIRTUAL_ELEMENT_POOL.lock_ref().iter().enumerate().map(|(pool_index, state)| {
                create_stable_variable_element(state.clone(), pool_index)
            })
        )
}

fn start_pool_update_task(
    variables: Vec<Signal>,
    visible_start: Mutable<usize>,
    visible_end: Mutable<usize>,
    item_height: f64
) {
    Task::start(async move {
        // Listen to visible range changes and update pool elements
        map_ref! {
            let start = visible_start.signal(),
            let end = visible_end.signal() => (*start, *end)
        }.for_each_sync(move |(start, end)| {
            
            let pool = VIRTUAL_ELEMENT_POOL.lock_ref();
            let visible_count = end - start;
            
            // Update each pool element
            for (pool_index, element_state) in pool.iter().enumerate() {
                let absolute_index = start + pool_index;
                
                if pool_index < visible_count && absolute_index < variables.len() {
                    // This element should be visible - update its content
                    if let Some(signal) = variables.get(absolute_index) {
                        // 🔥 CONTENT UPDATES - No element recreation!
                        element_state.name_signal.set_neq(signal.name.clone());
                        element_state.type_signal.set_neq(
                            format!("{} {}-bit", signal.signal_type, signal.width)
                        );
                        
                        // 🔥 POSITION UPDATES - Smooth repositioning!
                        element_state.position_signal.set_neq(
                            (absolute_index as f64 * item_height) as i32
                        );
                        
                        element_state.visible_signal.set_neq(true);
                        
                        // Set previous variable name for prefix highlighting
                        let previous_name = if absolute_index > 0 {
                            variables.get(absolute_index - 1).map(|prev_signal| prev_signal.name.clone())
                        } else {
                            None
                        };
                        element_state.previous_name_signal.set_neq(previous_name);
                    }
                } else {
                    // Hide this element by moving it off-screen
                    element_state.visible_signal.set_neq(false);
                    element_state.position_signal.set_neq(-9999);
                }
            }
        }).await;
    });
}

fn create_stable_variable_element(
    state: VirtualElementState,
    _pool_index: usize
) -> impl Element {
    
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        // 🔥 COMBINED POSITIONING + VISIBILITY - Single transform signal!
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
        .s(Background::new().color_signal(neutral_2()))
        .item(
            // 🔥 REACTIVE TEXT CONTENT - Only text nodes update!
            El::new()
                .s(Font::new().color_signal(neutral_11()).size(14))
                .s(Font::new().no_wrap())
                .child(Text::with_signal(state.name_signal.signal_cloned()))
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color_signal(primary_6()).size(12))
                .s(Font::new().no_wrap())
                .child(Text::with_signal(state.type_signal.signal_cloned()))
        )
}

// ===== HYBRID STABLE ELEMENT =====
// Optimized version for the hybrid MutableVec approach
fn create_stable_variable_element_hybrid(state: VirtualElementState) -> impl Element {
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
        .s(Background::new().color_signal(neutral_2()))
        .item(
            // Variable name with prefix highlighting
            create_variable_name_display(
                state.name_signal.signal_cloned(),
                state.previous_name_signal.signal_cloned(),
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

// LEGACY VERSION: For comparison - causes blank spaces
pub fn virtual_variable_row_positioned(signal: Signal, top_offset: f64, previous_name: Option<String>) -> impl Element {
    Row::new()
        .s(Gap::new().x(8))                                      // Horizontal spacing between elements
        .s(Width::fill())                                        // Full width within container
        .s(Height::exact(24))                                    // Fixed height per item (matches item_height)
        .s(Transform::new().move_down(top_offset as i32))        // CRITICAL: Absolute positioning within Stack
        .s(Padding::new().x(12).y(2))                           // Internal padding
        .s(Background::new().color_signal(neutral_2()))         // Row background color
        .item(
            // ===== VARIABLE NAME WITH PREFIX HIGHLIGHTING =====
            El::new()
                .s(Font::new().size(14))  // Text styling
                .child({
                    match &previous_name {
                        Some(prev) => {
                            let (prefix_len, has_prefix) = detect_shared_prefix(&signal.name, prev);
                            
                            if has_prefix && prefix_len > 0 {
                                let prefix = &signal.name[..prefix_len];
                                let suffix = &signal.name[prefix_len..];
                                
                                // Create paragraph with inline colored text
                                Paragraph::new()
                                    .content(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).no_wrap()) // Dimmed prefix
                                            .child(prefix)
                                    )
                                    .content(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_11()).no_wrap()) // Normal colored suffix
                                            .child(suffix)
                                    )
                                    .into_element()
                            } else {
                                // No shared prefix, display normally
                                Paragraph::new()
                                    .s(Font::new().color_signal(neutral_11()).no_wrap())
                                    .content(signal.name.clone())
                                    .into_element()
                            }
                        },
                        None => {
                            // First variable or no previous - display normally
                            Paragraph::new()
                                .s(Font::new().color_signal(neutral_11()).no_wrap())
                                .content(signal.name.clone())
                                .into_element()
                        }
                    }
                })
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color_signal(primary_6()).size(12))
                .s(Font::new().no_wrap())
                .child(format!("{} {}-bit", signal.signal_type, signal.width))
        )
}

#[allow(dead_code)]
pub fn virtual_variable_row(signal: Signal) -> impl Element {
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Padding::new().x(12).y(2))
        .s(Background::new().color_signal(neutral_2()))
        .item(
            El::new()
                .s(Font::new().color_signal(neutral_11()).size(14))
                .s(Font::new().no_wrap())
                .child(signal.name.clone())
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color_signal(primary_6()).size(12))
                .s(Font::new().no_wrap())
                .child(format!("{} {}-bit", signal.signal_type, signal.width))
        )
}

#[allow(dead_code)]
pub fn simple_variable_row(signal: Signal) -> Row<row::EmptyFlagNotSet, row::MultilineFlagNotSet, RawHtmlEl> {
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Padding::new().x(12).y(2))
        .item(
            El::new()
                .s(Font::new().color_signal(neutral_11()).size(14))
                .s(Font::new().no_wrap())
                .child(signal.name.clone())
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color_signal(primary_6()).size(12))
                .s(Font::new().no_wrap())
                .child(format!("{} {}-bit", signal.signal_type, signal.width))
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
                                
                                // Use Paragraph for inline text coloring
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
                                    .into_element()
                            } else {
                                // No shared prefix, display normally
                                Paragraph::new()
                                    .s(Font::new().color_signal(neutral_11()))
                                    .content(name.clone())
                                    .into_element()
                            }
                        },
                        None => {
                            // First item, no previous to compare with
                            Paragraph::new()
                                .s(Font::new().color_signal(neutral_11()))
                                .content(name.clone())
                                .into_element()
                        }
                    }
                }
            }
        )
}

// ===== SUPPORT FUNCTIONS =====

#[allow(dead_code)]
pub fn simple_variables_list(variables: Vec<Signal>, search_filter: String) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // Special case for displaying a message when called with empty variables and a message
    if variables.is_empty() && search_filter.starts_with("Select a scope") {
        return Column::new()
            .s(Gap::new().y(4))
            .s(Align::new().center_x())
            .s(Padding::new().top(32))
            .item(
                El::new()
                    .s(Font::new().color_signal(neutral_8()).size(13).italic())
                    .child(search_filter)
            );
    }
    
    // Apply search filter
    let filtered_variables = filter_variables(&variables, &search_filter);
    
    if variables.is_empty() {
        Column::new().item(empty_state_hint("No variables in selected scope"))
    } else if filtered_variables.is_empty() {
        Column::new().item(empty_state_hint("No variables match search filter"))
    } else {
        // Simple list showing all variables - clean and working
        Column::new()
            .s(Gap::new().y(0))
            .items(filtered_variables.into_iter().map(|signal| {
                simple_variable_row(signal)
            }))
    }
}

