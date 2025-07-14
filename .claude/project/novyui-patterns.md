# NovyUI Component Patterns

## Icon Design Tokens
- ALL components use `IconName` enum tokens, never magic strings
- Button: `button().left_icon(IconName::Folder)` 
- Input: `input().left_icon(IconName::Search)`
- Available icons: Check, X, Folder, Search, ArrowDownToLine, ZoomIn, ZoomOut, etc.
- Adding new icons requires: enum entry, to_kebab_case() mapping, SVG file mapping, string parsing
- Icon registry provides compile-time safety and IDE autocompletion

## Component Usage Patterns

### Button Component
```rust
button()
    .label("Click me")
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Medium)
    .left_icon(IconName::Folder)
    .align(Align::center())
    .on_press(|| { /* handler */ })
    .build()
```

### Input Component
```rust
input()
    .placeholder("Search...")
    .left_icon(IconName::Search)
    .size(InputSize::Small)
    .build()
```

### Panel Creation Helper
```rust
fn create_panel(header_content: impl Element, body_content: impl Element) -> impl Element {
    // Consistent panel styling
}
```

## Layout Patterns

### Header Layout (3-zone)
```rust
Row::new()
    .s(Gap::new().x(8))
    .s(Align::new().center_y())
    .item(title_text)
    .item(El::new().s(Width::fill()))  // spacer
    .item(right_button)
```

### Centered Button in Header
```rust
Row::new()
    .item(title)
    .item(El::new().s(Width::fill()).s(Align::center()).child(button))
    .item(right_button)
```

## Responsive Design Rules
- Use `Width::fill()` for responsive elements instead of `Width::exact()`
- Apply `Font::new().no_wrap()` to prevent text wrapping
- Use `Height::screen()` on root + `Height::fill()` on containers for full-screen layouts
- Gap sizing: `.x(1)` for tight spacing, `.x(8)` for normal spacing

## TreeView Component Patterns

### TreeView Background Width Patterns
**Problem:** TreeView item backgrounds don't extend to full content width in scrollable containers

**Solution A - Content-First (fit-content + min-width):**
```rust
.update_raw_el(|raw_el| {
    raw_el
        .style("width", "fit-content")
        .style("min-width", "100%")
})
```

**Solution B - Container-First (RECOMMENDED):**
```rust
.update_raw_el(|raw_el| {
    raw_el
        .style("min-width", "fit-content")
        .style("width", "100%")
})
```

**Recommendation:** Use Solution B (container-first) for TreeView items:
- Primary behavior: Fill panel width (better UX)
- Exception behavior: Expand for wide content (enables horizontal scroll)
- Semantic clarity: "always fill panel, expand only when content demands it"

### Scrollable Container Requirements
For panels containing TreeView with horizontal overflow:

```rust
// Panel scrollable container
El::new()
    .s(Scrollbars::both())
    .s(Width::fill())
    .update_raw_el(|raw_el| {
        raw_el.style("min-height", "0")      // Allow flex shrinking
              .style("overflow-x", "auto")   // Enable horizontal scroll
    })
```

**Key Insight:** Zoon `Width::fill()` + CSS `min-width: max-content` allows content to extend beyond container boundaries while maintaining responsive behavior.