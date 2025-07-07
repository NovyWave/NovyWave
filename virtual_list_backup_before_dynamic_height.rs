// BACKUP: Working fixed-height virtual list implementation
// Created before implementing dynamic height feature
// Location: frontend/src/main.rs lines 1484-1625 (approximately)
// Date: 2025-07-07

use zoon::*;

// Backup of working rust_virtual_variables_list function
fn rust_virtual_variables_list_backup(variables: Vec<Signal>) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    let total_items = variables.len();
    let item_height = 24.0;
    let container_height = Mutable::new(400.0); // Reactive height tracking
    let visible_count_value = ((400.0_f64 / item_height).ceil() as usize + 5).min(total_items);
    let visible_count = Mutable::new(visible_count_value);
    
    // Virtual scrolling state
    let scroll_top = Mutable::new(0.0);
    let visible_start = Mutable::new(0usize);
    let visible_end = Mutable::new(visible_count_value.min(total_items));
    
    zoon::println!("Virtual List: {} total, {} visible", total_items, visible_count_value);
    
    Column::new()
        .item(
            El::new()
                .s(Width::fill())
                .s(Height::exact(400))
                .s(Background::new().color(hsluv!(220, 15, 11)))
                .s(RoundedCorners::all(8))
                .s(Padding::all(4))
                .update_raw_el({
                    let scroll_top = scroll_top.clone();
                    let visible_start = visible_start.clone();
                    let visible_end = visible_end.clone();
                    let variables = variables.clone();
                    
                    move |el| {
                        // Setup scroll container
                        if let Some(html_el) = el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                            html_el.set_id("virtual-container");
                            html_el.style().set_property("overflow-y", "auto").unwrap();
                            
                            // Create scroll event handler
                            let scroll_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                                let scroll_top = scroll_top.clone();
                                let visible_start = visible_start.clone();
                                let visible_end = visible_end.clone();
                                
                                move |_event: web_sys::Event| {
                                    if let Some(scroll_el) = web_sys::window()
                                        .and_then(|w| w.document())
                                        .and_then(|d| d.get_element_by_id("virtual-container"))
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) 
                                    {
                                        let new_scroll_top = scroll_el.scroll_top() as f64;
                                        scroll_top.set_neq(new_scroll_top);
                                        
                                        let start_index = (new_scroll_top / item_height).floor() as usize;
                                        let end_index = (start_index + visible_count_value).min(total_items);
                                        
                                        visible_start.set_neq(start_index);
                                        visible_end.set_neq(end_index);
                                        
                                        zoon::println!("Scroll: top={}, start={}, end={}", new_scroll_top, start_index, end_index);
                                    }
                                }
                            }) as Box<dyn FnMut(_)>);
                            
                            // Add scroll event listener
                            html_el.add_event_listener_with_callback(
                                "scroll",
                                scroll_closure.as_ref().unchecked_ref()
                            ).unwrap();
                            
                            scroll_closure.forget();
                        }
                        
                        el
                    }
                })
                .child(
                    // Virtual scrollable content
                    El::new()
                        .s(Width::fill())
                        .s(Height::exact((total_items as f64 * item_height) as u32))
                        .child_signal(
                            map_ref! {
                                let start = visible_start.signal(),
                                let end = visible_end.signal() =>
                                // Create container with Stack and Transform positioning
                                Stack::new()
                                    .s(Width::fill())
                                    .s(Height::exact((total_items as f64 * item_height) as u32))
                                    .layers(
                                        variables[*start..*end].iter().enumerate().map(|(i, signal)| {
                                            let absolute_index = *start + i;
                                            virtual_variable_row_positioned_backup(signal.clone(), absolute_index as f64 * item_height)
                                        })
                                    )
                                    .into_element()
                            }
                        )
                )
        )
}

fn virtual_variable_row_positioned_backup(signal: Signal, top_offset: f64) -> impl Element {
    Row::new()
        .s(Gap::new().x(8))
        .s(Width::fill())
        .s(Height::exact(24))
        .s(Transform::new().move_down(top_offset as i32))
        .s(Padding::new().x(12).y(2))
        .s(Background::new().color(hsluv!(220, 15, 12)))
        .item(
            El::new()
                .s(Font::new().color(hsluv!(220, 10, 85)).size(14))
                .s(Font::new().no_wrap())
                .child(signal.name.clone())
        )
        .item(El::new().s(Width::fill()))
        .item(
            El::new()
                .s(Font::new().color(hsluv!(210, 80, 70)).size(12))
                .s(Font::new().no_wrap())
                .child(format!("{} {}-bit", signal.signal_type, signal.width))
        )
}

// KEY IMPLEMENTATION DETAILS:
// 1. Fixed height: 400px (.s(Height::exact(400)))
// 2. Fixed container_height: Mutable::new(400.0)
// 3. Fixed visible_count calculation based on 400px
// 4. Uses Transform::new().move_down() for positioning
// 5. Stack with layers for virtual items
// 6. JavaScript closure for scroll event handling
// 7. Manual DOM manipulation via update_raw_el()

// INTEGRATION POINTS FOR DYNAMIC HEIGHT:
// - Replace Height::exact(400) with Height::fill() or dynamic height
// - Add on_viewport_size_change() to track container size changes
// - Make container_height reactive to actual container size
// - Recalculate visible_count when height changes
// - Ensure scroll state remains consistent during height changes