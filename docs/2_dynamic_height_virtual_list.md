# Dynamic Height Virtual List Implementation Research

## Overview

Research findings for implementing dynamic height virtual lists in MoonZoon/Zoon framework for NovyWave waveform viewer.

## Key MoonZoon/Zoon Viewport & Resize Detection Methods

### 1. **ResizableViewport Trait**
- **Method**: `on_viewport_size_change(handler: impl FnMut(U32Width, U32Height) + 'static)`
- **Purpose**: Detects when an element's size changes using ResizeObserver API
- **Implementation**: Uses native ResizeObserver for precise element dimension tracking
- **Elements**: El, Column, Row, Paragraph, Spacer, Stack, Stripe, Grid

### 2. **MutableViewport Trait**  
- **Method**: `on_viewport_location_change(handler: impl FnMut(Scene, Viewport) + 'static)`
- **Purpose**: Detects scroll position changes within scrollable containers
- **Provides**: Scene (scroll_width, scroll_height) and Viewport (scroll_left, scroll_top, client_width, client_height)
- **Implementation**: Uses scroll events to track viewport position
- **Elements**: El, Column, Row, Paragraph, Spacer, Stack, Stripe, Grid

### 3. **Viewport Signal Methods**
- **Methods**: `viewport_x_signal()`, `viewport_y_signal()`
- **Purpose**: Programmatically control scroll position
- **Usage**: Set scroll position from signals

## Example Usage Patterns

### Basic Height Monitoring
```rust
// Monitor element size changes
El::new()
    .on_viewport_size_change(|width, height| {
        // Element dimensions changed
        println!("New size: {}x{}", width, height);
    })
```

### Scroll Position Tracking
```rust
// Monitor scroll position in scrollable container
Column::new()
    .s(Scrollbars::both())
    .on_viewport_location_change(|scene, viewport| {
        // scene: total scrollable area
        // viewport: visible area and scroll position
        let visible_start = viewport.y;
        let visible_end = viewport.y + viewport.height as i32;
    })
```

## ResizeObserver Implementation Details

- **Native API**: Uses browser's ResizeObserver API for optimal performance
- **Cross-browser**: Handles Safari/iOS fallback using contentRect 
- **Automatic cleanup**: Observer disconnects when element is dropped
- **Precision**: Provides exact pixel dimensions (u32 width/height)

## Research Findings from MoonZoon Examples

### 1. **Viewport Example** (`/examples/viewport/`)
- Shows complete viewport change detection pattern
- Uses global signals for viewport state management
- Demonstrates scroll position control with signals
- Pattern: `on_viewport_location_change(|_, viewport| emit(viewport))`

### 2. **Pan/Zoom Example** (`/examples/pan_zoom/`)
- Shows manual resize detection using `get_bounding_client_rect()`
- Comments mention potential `.on_resize` usage for SVG elements
- Demonstrates DOM-based dimension tracking as ResizeObserver alternative

### 3. **Video Example** (`/examples/video/`)
- Shows awareness of element height changes from content (poster images)
- Demonstrates patterns for dynamic content affecting layout

## Virtual List Implementation Strategy

### Core Pattern
```rust
// Container that monitors its own height changes
Column::new()
    .s(Height::fill())
    .s(Scrollbars::both())
    .on_viewport_size_change(|width, height| {
        // Update available space for virtual list
        update_container_height(height);
    })
    .on_viewport_location_change(|scene, viewport| {
        // Calculate visible range
        let visible_start = viewport.y;
        let visible_end = viewport.y + viewport.height as i32;
        update_visible_range(visible_start, visible_end);
    })
    .items_signal(virtual_items_signal)
```

### Height Cache Management
```rust
// Individual items report their height after render
fn list_item(item: ItemData) -> impl Element {
    El::new()
        .on_viewport_size_change(move |_, height| {
            // Cache this item's height
            cache_item_height(item.id, height);
            // Trigger virtual list recalculation
            update_virtual_list();
        })
        .child(item_content(item))
}
```

## Technical Implementation Notes

### ResizeObserver Performance
- **Global Observer**: MoonZoon creates individual observers per element
- **Optimization Note**: Code comments suggest potential for single global observer
- **Current Approach**: Each element gets its own ResizeObserver instance

### Browser Compatibility
- **Modern Browsers**: Uses `borderBoxSize` for precise measurements
- **Safari/iOS**: Falls back to `contentRect` dimensions
- **Automatic Detection**: Runtime feature detection handles differences

### Memory Management
- **Automatic Cleanup**: Observers disconnect on element removal
- **Closure Handling**: Proper cleanup of Rust closures for callbacks
- **Signal Integration**: Works with Zoon's signal system for reactive updates

## Implementation Plan

### Phase 1: Basic Height Monitoring
1. Create container with `on_viewport_size_change` for container sizing
2. Add `on_viewport_location_change` for scroll tracking
3. Implement basic visible range calculation

### Phase 2: Item Height Caching
1. Add `on_viewport_size_change` to individual items
2. Build height cache with item ID mapping
3. Implement recalculation triggers

### Phase 3: Virtual Rendering
1. Calculate visible items based on cached heights
2. Add estimated heights for unmeasured items
3. Implement smooth scrolling with proper positioning

### Phase 4: Optimization
1. Debounce rapid resize events
2. Implement incremental height updates
3. Add performance monitoring

## Potential Challenges

### 1. **Height Estimation**
- **Problem**: Need heights before items are rendered
- **Solution**: Use average height estimation + progressive measurement

### 2. **Scroll Position Accuracy**
- **Problem**: Changing item heights affect scroll position
- **Solution**: Track accumulated height changes and adjust scroll

### 3. **Performance with Many Items**
- **Problem**: Many ResizeObserver instances
- **Solution**: Consider implementing custom global observer

### 4. **Signal Coordination**
- **Problem**: Multiple signals need synchronization
- **Solution**: Use centralized virtual list state management

## References

- **MoonZoon Examples**: `/examples/viewport/`, `/examples/pan_zoom/`
- **Source Code**: `/crates/zoon/src/element/ability/resizable_viewport.rs`
- **ResizeObserver**: `/crates/zoon/src/resize_observer.rs`
- **Native API**: Browser ResizeObserver documentation

## Next Steps

1. Implement basic prototype with fixed-height items
2. Add ResizeObserver monitoring to container
3. Implement height caching for individual items
4. Test with NovyWave's waveform data structures
5. Optimize for performance with large datasets

## Notes for Future Development

- Consider implementing global ResizeObserver for better performance
- Monitor for MoonZoon framework updates to viewport APIs
- Test extensively with various screen sizes and zoom levels
- Ensure proper cleanup of observers and signals