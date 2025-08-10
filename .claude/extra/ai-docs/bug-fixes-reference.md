# Bug Fixes Reference

## Compilation Errors and Fixes

### WASM Compilation Issues
**Problem**: Frontend changes not visible despite code changes
**Root Cause**: MoonZoon only auto-reloads after successful compilation
**Fix**: Always check compilation first
```bash
tail -100 dev_server.log | grep -i "error"
```

**Problem**: cargo build/check showing no WASM errors but mzoon failing
**Root Cause**: cargo cannot check WASM compilation properly
**Fix**: Only trust mzoon output for WASM build status

### Icon Compilation Errors
**Problem**: `IconName::Check` causes compilation errors
**Root Cause**: Missing IconName enum variant or incorrect usage
**Fix**: Always use IconName enum tokens, check available variants
```rust
// Correct usage
button().left_icon(IconName::Check)

// Check available icons in novyui/src/icon.rs
pub enum IconName {
    Check, X, Folder, Search, ArrowDownToLine, // etc.
}
```

### Signal Type Mismatches
**Problem**: `El.item_signal()` API compatibility errors
**Root Cause**: Signal type unification issues
**Fix**: Use `.into_element()` for type unification
```rust
.item_signal(content_signal.map(|content| {
    match content {
        ContentType::A => element_a().into_element(),
        ContentType::B => element_b().into_element(),
    }
}))
```

## Layout and Styling Problems

### TreeView Width Issues
**Problem**: TreeView item backgrounds don't extend to full content width
**Root Cause**: Multiple levels of width constraints
**Ultimate Fix**: Multi-level constraint solution
```rust
// Container level
El::new()
    .s(Width::fill())
    .update_raw_el(|raw_el| {
        raw_el.style("min-width", "max-content")  // Horizontal expansion
    })

// Button level  
Button::new()
    .s(Width::fill())
    
// CSS level
.update_raw_el(|raw_el| {
    raw_el
        .style("width", "100%")
        .style("box-sizing", "border-box")
})
```

### Height Inheritance Chain Breaks
**Problem**: Containers not filling available height
**Root Cause**: Missing `Height::fill()` anywhere in the chain
**Fix**: Ensure every container in hierarchy has `Height::fill()`
```rust
El::new().s(Height::screen())              // Root
  .child(Column::new().s(Height::fill())   // Every container needs this
    .item(Row::new().s(Height::fill())     // Missing this breaks everything
      .item(content)))
```

### Scrollable Container Issues
**Problem**: Content not scrolling properly in flexbox layouts
**Root Cause**: Parent containers don't allow shrinking
**Fix**: Add `min-height: 0` to parent containers
```rust
.update_raw_el(|raw_el| {
    raw_el
        .style("min-height", "0")      // Allow flex shrinking
        .style("overflow-x", "auto")   // Enable horizontal scroll
})
```

### Dropdown Height Problems
**Problem**: Dropdowns show scrollbars for small content
**Root Cause**: Invisible newline characters causing multi-line rendering
**Fix**: Unicode character filtering
```rust
use unicode_width::UnicodeWidthChar;

let clean_text: String = text.chars()
    .filter(|&c| c == ' ' || UnicodeWidthChar::width(c).unwrap_or(0) > 0)
    .collect();
```

## Event Handling Issues

### Checkbox Event Bubbling
**Problem**: Checkbox clicks trigger both selection and expansion
**Root Cause**: Event propagation to parent row click handler
**Fix**: Prevent event propagation
```rust
El::new()
    .child(checkbox)
    .on_hovered_change(/* ... */)
    .global_event_handler(move |event: PointerDown| {
        event.pass_to_parent(false);  // Prevent bubbling
    })
```

### Canvas Click Coordinate Issues
**Problem**: Click events use wrong coordinate system
**Root Cause**: Page coordinates vs canvas-relative coordinates
**Fix**: Convert using getBoundingClientRect
```rust
let relative_x = event.client_x() as f32 - canvas_rect.left();
let relative_y = event.client_y() as f32 - canvas_rect.top();
```

### Global Keyboard Handlers
**Problem**: Keyboard events not working for modal dialogs
**Root Cause**: Focus management complexity
**Fix**: Use global event handlers with state guards
```rust
.global_event_handler({
    let close_dialog = close_dialog.clone();
    move |event: KeyDown| {
        if DIALOG_IS_OPEN.get() {  // Guard with dialog state
            if event.key() == "Escape" {
                close_dialog();
            }
        }
    }
})
```

## Memory Management Problems

### Session Storage Issues  
**Problem**: Large session data causing storage problems
**Root Cause**: Excessive data storage in single observations
**Fix**: Use separate log files for large data, 2KB limits for observations
```bash
# Hook implementation with size limits
if [[ ${#data} -gt 2048 ]]; then
    echo "$data" > "$PROJECT_ROOT/.claude/logs/large-data-$(date +%s).log"
    echo "Large data stored in separate log file" # Store small reference instead
fi
```

### Config Loading Race Conditions
**Problem**: Startup config overwrites user settings
**Root Cause**: Reactive triggers starting before config loads
**Fix**: Use initialization order with gate flags
```rust
// Load config first, then start reactive triggers
async fn initialize_app() {
    load_config().await;
    CONFIG_LOADED.set_neq(true);  // Gate flag
    init_reactive_handlers();     // Start triggers after config loads
}

// Guard reactive operations
if CONFIG_LOADED.get() {
    perform_config_operation();
}
```

## Configuration Persistence Issues

### MutableVec Signal Chain Breaks
**Problem**: Complex signal chains don't trigger config saves
**Root Cause**: Multiple signal transformations break reactive triggers
**Fix**: Manual save calls in sync functions
```rust
fn sync_globals_to_config() {
    // Update config store
    let items: Vec<_> = GLOBAL_STATE.lock_ref().to_vec();
    CONFIG_STORE.with(|store| store.items.set_neq(items));
    
    // Manual save trigger when reactive signals fail
    save_config_to_backend();
}
```

### Dock Mode Panel Dimension Issues
**Problem**: Panel heights getting overwritten between dock modes
**Root Cause**: Semantic overloading - same field controls different panels
**Fix**: Separate storage per dock mode
```rust
// Wrong - semantic overloading
pub struct Config {
    pub files_panel_height: f32,  // Means different things in different modes
}

// Correct - explicit per-mode storage
pub struct Config {
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
}
```

### Scope Selection Persistence
**Problem**: Selected scope lost on app restart despite being stored
**Root Cause**: Missing field in shared crate for backend persistence
**Fix**: Add field to both frontend and shared crate
```rust
// shared/lib.rs
pub struct WorkspaceSection {
    pub selected_scope_id: Option<String>,  // Add to backend schema
}

// frontend/config.rs - already exists, just needs backend sync
```

## Performance Issues

### Virtual List Blank Spaces
**Problem**: Empty spaces during scrolling in virtual lists
**Root Cause**: Element recreation during scroll events
**Fix**: Stable element pools with content-only updates
```rust
// Use stable element pool
let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(
    (0..buffer_size).map(|_| VirtualElementState::default()).collect()
);

// Update content only, never recreate elements
element.text_signal.set_neq(new_content);
element.position_signal.set_neq(new_position);
```

### Excessive Debug Logging
**Problem**: Development server logs unreadable due to debug spam
**Root Cause**: Excessive println! statements in tight loops
**Fix**: Systematic debug cleanup
```bash
# Find and remove debug statements
rg "println!" --type rust | wc -l    # Count debug statements
# Remove non-essential debug output, keep only error logging
```

### Directory Scanning Performance
**Problem**: Load Files dialog extremely slow on large directories
**Root Cause**: Synchronous directory traversal with full recursive scans
**Fix**: Parallel traversal with jwalk
```rust
use jwalk::WalkDir;

WalkDir::new(path)
    .parallelism(jwalk::Parallelism::RayonNewPool(4))
    .into_iter()
    .filter_map(|entry| entry.ok())
    .collect()  // 4x performance improvement
```