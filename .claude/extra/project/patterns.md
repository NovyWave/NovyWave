# Framework Patterns & UI Components

## MoonZoon Framework Configuration

### Framework Overview
MoonZoon is a Rust-based full-stack web framework using:
- **Frontend:** Rust + WASM using Zoon UI framework
- **Backend:** Moon server framework (optional)
- **Build Tool:** mzoon CLI

### Development Commands
**Standard Commands:**
- `makers start` - Start development server with auto-reload
- `makers build` - Production build
- `makers install` - Install dependencies
- `makers clean` - Clean build artifacts

**Desktop Support (if using Tauri):**
- `makers tauri` - Start Tauri desktop development
- `makers tauri-build` - Build desktop application

### Critical WASM Rules
**Compilation:**
- Use `zoon::println!()` for logging, NOT `std::println!()`
- NEVER use `cargo build` or `cargo check` - only mzoon handles WASM properly
- Auto-reload only triggers after successful compilation

**Development Workflow:**
- Run `makers start > dev_server.log 2>&1 &` as BACKGROUND PROCESS
- Monitor compilation with `tail -f dev_server.log`
- Read compilation errors from mzoon output for accurate WASM build status
- Browser auto-reloads ONLY after successful compilation

## Zoon Framework Patterns

### Layout Fundamentals

#### Height Inheritance Pattern
```rust
// Root element claims viewport
El::new().s(Height::screen())

// All containers must inherit
Column::new().s(Height::fill())
Row::new().s(Height::fill())
```

**Critical:** Missing `Height::fill()` in any container breaks the height inheritance chain.

#### Responsive Width
```rust
// Good - responsive
Row::new().s(Width::fill())

// Bad - fixed width causes overflow
Row::new().s(Width::exact(800))
```

#### Spacing and Alignment
```rust
// Vertical centering in headers
.s(Align::new().center_y())

// Horizontal spacing
.s(Gap::new().x(8))  // normal spacing
.s(Gap::new().x(1))  // tight spacing

// Spacer element
El::new().s(Width::fill())
```

### Signal-Based Layouts

#### Dynamic Layout Switching
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

### Common Layout Patterns

#### Full-Screen Layout
```rust
fn root() -> impl Element {
    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .child(main_layout())
}
```

#### Panel with Header
```rust
fn panel_with_header(title: &str, content: impl Element) -> impl Element {
    Column::new()
        .s(Height::fill())
        .item(header_row(title))
        .item(content)
}
```

### Event Handling Patterns

#### Global Keyboard Event Handler
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

### Debug Techniques
Use bright background colors on containers to visualize height inheritance:
```rust
.s(Background::new().color(Color::red()))  // Debug only
```

## NovyUI Component Patterns

### Icon Design Tokens
- ALL components use `IconName` enum tokens, never magic strings
- Button: `button().left_icon(IconName::Folder)` 
- Input: `input().left_icon(IconName::Search)`
- Available icons: Check, X, Folder, Search, ArrowDownToLine, ZoomIn, ZoomOut, etc.
- Adding new icons requires: enum entry, to_kebab_case() mapping, SVG file mapping, string parsing
- Icon registry provides compile-time safety and IDE autocompletion

### Component Usage Patterns

#### Button Component
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

#### Input Component
```rust
input()
    .placeholder("Search...")
    .left_icon(IconName::Search)
    .size(InputSize::Small)
    .build()
```

### Layout Patterns

#### Header Layout (3-zone)
```rust
Row::new()
    .s(Gap::new().x(8))
    .s(Align::new().center_y())
    .item(title_text)
    .item(El::new().s(Width::fill()))  // spacer
    .item(right_button)
```

#### Centered Button in Header
```rust
Row::new()
    .item(title)
    .item(El::new().s(Width::fill()).s(Align::center()).child(button))
    .item(right_button)
```

### Responsive Design Rules
- Use `Width::fill()` for responsive elements instead of `Width::exact()`
- Apply `Font::new().no_wrap()` to prevent text wrapping
- Use `Height::screen()` on root + `Height::fill()` on containers for full-screen layouts
- Gap sizing: `.x(1)` for tight spacing, `.x(8)` for normal spacing

### TreeView Component Patterns

#### TreeView Background Width Patterns
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

#### Scrollable Container Requirements
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

## Development & Debugging

### WASM/Frontend Development Process

**CRITICAL WORKFLOW:**
- **ABSOLUTE PROHIBITION: NEVER restart MoonZoon dev server without explicit user permission**
- **MANDATORY: ALWAYS ask user to use /project-stop or /project-start commands**
- **PATIENCE REQUIREMENT: Backend/shared crate compilation takes DOZENS OF SECONDS TO MINUTES**
- **WAIT ENFORCEMENT: You MUST wait for compilation to complete, no matter how long it takes**
- **COMPILATION MONITORING ONLY:** Monitor with `tail -f dev_server.log` - DO NOT manage processes
- **Clear dev_server.log when it gets too long** - use `> dev_server.log` to truncate
- **BROWSER TESTING PROTOCOL:** Always test changes with browser MCP after making changes
- **READ ERRORS, DON'T RESTART:** Read compilation errors, don't restart repeatedly
- **CARGO PROHIBITION:** NEVER use `cargo build` or `cargo check` - they cannot check WASM compilation
- **MZOON OUTPUT ONLY:** Only trust mzoon output for accurate WASM build status

### Debug Patterns
- Use `zoon::println!()` for console logging, NOT `std::println!()` (which does nothing in WASM)
- All frontend code compiles to WebAssembly and runs in browser environment
- For development verification, use the three built-in examples: Simple Rectangle, Face with Hat, and Sine Wave

### Advanced UI Debugging Techniques

**Auto-Scroll Testing for Width Issues:**
- Create `Task::start + Timer::sleep + viewport_x_signal + i32::MAX` to reveal horizontal layout problems
- Essential for debugging TreeView, table, and scrollable content width constraints
- Allows testing width behavior that's invisible in normal view

**Multi-Subagent Problem Solving:**
- Fire 3+ specialized subagents simultaneously for complex UI issues
- Pattern: (1) Browser DOM/CSS inspection agent (2) Minimal test case creation agent (3) Comprehensive solution research agent
- Each agent provides focused expertise while main session coordinates and implements
- Use TodoWrite for systematic task breakdown and progress tracking

**Width Constraint Debugging:**
- Common issue: TreeView/component backgrounds don't extend to full content width in scrollable containers
- Root cause: Multiple levels of width constraints (container → item → CSS)
- Solution pattern: Container needs `Width::fill() + CSS min-width: max-content` + Items need `Width::fill()` + CSS needs `width: 100%`