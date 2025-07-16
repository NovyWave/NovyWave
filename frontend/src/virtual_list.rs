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
                                                // Position each item using Transform (absolute positioning)
                                                virtual_variable_row_positioned(signal.clone(), absolute_index as f64 * item_height)
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
    
    // ===== STEP 2: REACTIVE VISIBLE COUNT =====
    // This will be updated when height changes (Step 3 will connect it)
    let visible_count = Mutable::new(initial_visible_count);
    
    // ===== STEP 3: HEIGHT SIGNAL LISTENER =====
    // Listen to height changes and recalculate visible count
    Task::start({
        let height_signal = height_signal.clone();
        let visible_count = visible_count.clone();
        async move {
            height_signal.signal().for_each(|height| {
                let new_visible_count = ((height as f64 / item_height).ceil() as usize + 5).min(total_items);
                visible_count.set_neq(new_visible_count);
                async {}
            }).await;
        }
    });
    
    // ===== STEP 4: UPDATE visible_end WHEN visible_count CHANGES =====
    // When visible_count changes, update visible_end to maintain current view
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
                    
                    move |el| {
                        if let Some(html_el) = el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                            html_el.set_id("virtual-container-signal");
                            html_el.style().set_property("overflow-y", "auto").unwrap();
                            html_el.style().set_property("display", "block").unwrap();
                            
                            
                            let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                                let scroll_top = scroll_top.clone();
                                let visible_start = visible_start.clone();
                                let visible_end = visible_end.clone();
                                
                                move |_event: web_sys::Event| {
                                    if let Some(scroll_el) = web_sys::window()
                                        .and_then(|w| w.document())
                                        .and_then(|d| d.get_element_by_id("virtual-container-signal"))
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) 
                                    {
                                        let new_scroll_top = scroll_el.scroll_top() as f64;
                                        scroll_top.set_neq(new_scroll_top);
                                        
                                        let start_index = (new_scroll_top / item_height).floor() as usize;
                                        // STEP 4: Use reactive visible_count instead of static initial_visible_count
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
                    El::new()
                        .s(Width::fill())
                        .s(Height::exact((total_items as f64 * item_height) as u32))
                        .child(
                            // 🔥🔥🔥 REVOLUTIONARY CHANGE: STABLE ELEMENT POOL! 🔥🔥🔥
                            // ZERO DOM recreation during scroll - only content/position updates!
                            create_stable_virtual_list(
                                variables.clone(),
                                visible_start.clone(),
                                visible_end.clone(),
                                item_height,
                                total_items
                            )
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

// ===== REVOLUTIONARY STABLE ELEMENT POOL VIRTUALIZATION =====
// ZERO DOM RECREATION - Only content and position updates via signals!

#[derive(Clone)]
struct VirtualElementState {
    name_signal: Mutable<String>,
    type_signal: Mutable<String>,
    position_signal: Mutable<i32>,
    visible_signal: Mutable<bool>,
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

// LEGACY VERSION: For comparison - causes blank spaces
pub fn virtual_variable_row_positioned(signal: Signal, top_offset: f64) -> impl Element {
    Row::new()
        .s(Gap::new().x(8))                                      // Horizontal spacing between elements
        .s(Width::fill())                                        // Full width within container
        .s(Height::exact(24))                                    // Fixed height per item (matches item_height)
        .s(Transform::new().move_down(top_offset as i32))        // CRITICAL: Absolute positioning within Stack
        .s(Padding::new().x(12).y(2))                           // Internal padding
        .s(Background::new().color_signal(neutral_2()))         // Row background color
        .item(
            // ===== VARIABLE NAME =====
            El::new()
                .s(Font::new().color_signal(neutral_11()).size(14))  // Text styling
                .s(Font::new().no_wrap())                             // Prevent text wrapping
                .child(signal.name.clone())                           // Display variable name
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

