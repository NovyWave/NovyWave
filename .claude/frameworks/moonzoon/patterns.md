# Zoon Framework Patterns

## Layout Fundamentals

### Height Inheritance Pattern
```rust
// Root element claims viewport
El::new().s(Height::screen())

// All containers must inherit
Column::new().s(Height::fill())
Row::new().s(Height::fill())
```

**Critical:** Missing `Height::fill()` in any container breaks the height inheritance chain.

### Responsive Width
```rust
// Good - responsive
Row::new().s(Width::fill())

// Bad - fixed width causes overflow
Row::new().s(Width::exact(800))
```

### Spacing and Alignment
```rust
// Vertical centering in headers
.s(Align::new().center_y())

// Horizontal spacing
.s(Gap::new().x(8))  // normal spacing
.s(Gap::new().x(1))  // tight spacing

// Spacer element
El::new().s(Width::fill())
```

## Signal-Based Layouts

### Dynamic Layout Switching
```rust
// Use Stripe for Row/Column switching
Stripe::new()
    .direction_signal(layout_signal.map(|mode| {
        if mode.is_docked() { Direction::Column } else { Direction::Row }
    }))
    .item_signal(content_signal.map(|content| {
        match content {
            ContentType::A => element_a().into_element(),
            ContentType::B => element_b().into_element(),
        }
    }))
```

**Important:** Use `.into_element()` to unify types when using if/else branches in signals.

## Common Patterns

### Full-Screen Layout
```rust
fn root() -> impl Element {
    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .child(main_layout())
}
```

### Panel with Header
```rust
fn panel_with_header(title: &str, content: impl Element) -> impl Element {
    Column::new()
        .s(Height::fill())
        .item(header_row(title))
        .item(content)
}
```

### Debug Technique
Use bright background colors on containers to visualize height inheritance:
```rust
.s(Background::new().color(Color::red()))  // Debug only
```

## Event Handling Patterns

### Global Keyboard Event Handler
For dialog keyboard accessibility without focus management:
```rust
.global_event_handler({
    let close_dialog = close_dialog.clone();
    move |event: KeyDown| {
        if DIALOG_IS_OPEN.get() {  // Guard with dialog state
            if event.key() == "Escape" {
                close_dialog();
            } else if event.key() == "Enter" {
                process_selection();
            }
        }
    }
})
```

**Why Global Event Handlers:**
- Work immediately when dialog opens (no focus required)
- Capture events at document level
- Better than autofocus or local element handlers for modal dialogs
- Essential for keyboard accessibility in Zoon applications

**Pattern:** Always guard global handlers with state checks to prevent interference with other UI components.