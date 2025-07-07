# UI Reactive Patterns - Lessons from Dynamic Virtual List Implementation

## Broadcaster Pattern for Signal Cloneability

**Problem**: `MutableSignal<T>` doesn't implement `Clone`, but we need to share signals across multiple contexts.

**Solution**: Use `Broadcaster<MutableSignal<T>>` to make signals cloneable.

```rust
// ❌ This doesn't work - MutableSignal<T> is not Clone
let height_signal = Mutable::new(400u32).signal();
let cloned_signal = height_signal.clone(); // ERROR

// ✅ Use Broadcaster to make signals cloneable
let height_mutable = Mutable::new(400u32);
let height_signal = Broadcaster::new(height_mutable.signal());

// Now you can clone and pass around
let signal_copy = height_signal.clone();
rust_virtual_list_with_signal(variables, height_signal);
```

**When to use**: 
- Signal needs to be shared across multiple functions/closures
- Parent-child communication where child needs signal access
- Complex reactive architectures with signal branching

## `into_element()` Type Unification Pattern

**Problem**: When using conditional rendering with `if/else` branches that return different element types, Rust can't unify the types.

**Solution**: Use `.into_element()` to convert all branches to unified `Element` trait object.

```rust
// ❌ This fails - different types from if/else branches
let content = if condition {
    Column::new().item("Column content")  // Returns Column<...>
} else {
    Row::new().item("Row content")        // Returns Row<...> - DIFFERENT TYPE!
};

// ✅ Use into_element() to unify types
let content = if condition {
    Column::new().item("Column content").into_element()  // Returns Element
} else {
    Row::new().item("Row content").into_element()        // Returns Element
};

// ✅ Also works with signals and map
content_signal.map(|content_type| {
    match content_type {
        ContentType::List => list_component().into_element(),
        ContentType::Grid => grid_component().into_element(),
        ContentType::Table => table_component().into_element(),
    }
})
```

**Return Type Patterns**:
- `impl Element` - for single known type returns
- `Element` (trait object) - for dynamic/conditional returns  
- `RawHtmlEl` - for specific HTML element returns

## Height::fill() Hierarchy Chain Pattern

**Critical Rule**: Height::fill() requires an unbroken chain from root to target.

```rust
// ❌ Broken chain - missing Height::fill() on Column
Column::new()                           // Missing .s(Height::fill())
    .item(
        El::new().s(Height::fill())     // This won't work - parent doesn't fill
    )

// ✅ Complete chain - all parents have Height::fill()
Column::new().s(Height::fill())         // Parent fills
    .item(
        El::new().s(Height::fill())     // Child can now fill parent
    )
```

**Debug technique**: Add bright background colors to visualize hierarchy:
```rust
.s(Background::new().color(Color::red()))  // Temporary debug only
```

## Parent-Child Wrapper Pattern for Responsive + Precise Layouts

**Use case**: When you need responsive sizing but child component requires exact dimensions.

```rust
// PARENT: Responsive sizing (adapts to available space)
Column::new().s(Height::fill())
    .item(
        // MONITOR: Measures actual space available
        El::new()
            .s(Height::fill())
            .on_viewport_size_change(move |_width, height| {
                height_signal.set_neq(height);
            })
            .child(
                // CHILD: Uses exact measured dimensions
                virtual_list_with_signal(data, height_signal)
            )
    )
```

**Pattern benefits**:
- Separates responsive concerns from precise rendering
- Enables virtual scrolling (needs exact heights) within responsive layouts
- Real-time measurement without breaking layout flow

## Signal-Based Reactive Debugging Pattern

**Always add diagnostic logging to signal chains:**

```rust
Task::start({
    let signal = signal.clone();
    async move {
        signal.for_each(|value| {
            zoon::println!("Signal changed: {} -> processing", value);
            // Your reactive logic here
            async {}
        }).await;
    }
});
```

**Browser console debugging**:
- Use `zoon::println!()` not `std::println!()` in WASM
- Console logs reveal reactive flow in real-time
- Better than IDE diagnostics for runtime reactive behavior

## Virtual Scrolling + Dynamic Height Architecture

**Key insight**: Virtual lists need exact heights, but responsive layouts need Height::fill().

```rust
// ❌ Direct Height::fill() on virtual list breaks scrolling (clientHeight=0)
virtual_list().s(Height::fill())

// ✅ Signal bridge pattern connects responsive to precise
let height_mutable = Mutable::new(400u32);
let height_broadcaster = Broadcaster::new(height_mutable.signal());

// Parent measures, child uses exact signal
parent_with_monitoring(height_mutable) // Updates height_mutable
    .child(virtual_list_with_signal(height_broadcaster)) // Uses exact signal
```

## Incremental Complex UI Development

**Strategy for complex UI changes**:

1. **Break into testable steps** - each should work or fail predictably
2. **Test performance after each step** - catch slowdowns early
3. **Add TODO tracking** - maintain visibility of progress
4. **Console logging** - verify each step's behavior
5. **Screenshot verification** - visual confirmation of changes

```rust
// Example: Step-by-step dynamic height implementation
// Step 1: Remove height constraints (test: allows taller panels)
// Step 2: Add reactive infrastructure (test: no behavior change)  
// Step 3: Connect signals (test: see console logs)
// Step 4: Update rendering (test: empty space fixed)
// Step 5: Performance testing (test: smooth at all sizes)
```

## Signal Listener Cleanup Pattern

**For reactive architectures with multiple signal listeners:**

```rust
// Clean separation of concerns
Task::start({ // Height changes -> visible count
    height_signal.signal().for_each(|height| {
        let new_count = calculate_visible_count(height);
        visible_count.set_neq(new_count);
        async {}
    }).await;
});

Task::start({ // Visible count changes -> update rendering
    visible_count.signal().for_each(|count| {
        let new_end = visible_start.get() + count;
        visible_end.set_neq(new_end);
        async {}
    }).await;
});
```

## Performance-First Reactive Design

**Principles for high-performance reactive UIs**:

- **Only render visible + buffer** (not all items)
- **Recalculate on state changes**, not continuously  
- **Signal-based updates** over polling/timers
- **Measure performance at each step** of complex implementations
- **Constraint expensive operations** (limit height ranges if needed)

## WASM-Specific UI Patterns

**Common WASM gotchas and solutions**:

```rust
// ❌ std::println!() - doesn't work in WASM
std::println!("Debug info");

// ✅ zoon::println!() - works in WASM  
zoon::println!("Debug info");

// ❌ clientHeight=0 with certain CSS patterns
el.s(Height::fill()) // May result in clientHeight=0

// ✅ Use exact heights for scroll containers
el.s(Height::exact_signal(height_signal))
```

**Debugging approach**:
- Browser DevTools console > IDE errors for runtime issues
- Visual verification with screenshots
- Height measurement issues manifest differently in WASM

## Incremental Implementation Strategy

**4-Phase approach for complex UI changes:**

```rust
// Phase 1: Add monitoring without changing behavior
.on_viewport_size_change(|_width, height| {
    zoon::println!("Height changed: {}", height); // Observe only
})

// Phase 2: Add reactive infrastructure
let visible_count = Mutable::new(initial_count);

// Phase 3: Connect signals
Task::start({
    height_signal.signal().for_each(|height| {
        let new_count = calculate_visible_count(height);
        visible_count.set_neq(new_count);
        async {}
    }).await;
});

// Phase 4: Update rendering logic
let end_index = (start_index + visible_count.get()).min(total_items);
```

**Benefits:** Each phase is independently testable, enables precise rollback, prevents breaking working functionality.

## Risk Mitigation Patterns

**Height Change During User Interaction:**
```rust
// Preserve scroll percentage instead of absolute position
let scroll_percentage = scroll_top / max_scroll_height;
// After height change:
let new_scroll_top = scroll_percentage * new_max_scroll_height;
```

**Rapid Changes (Panel Resizing):**
- Use `set_neq()` to prevent unnecessary updates
- Debounce viewport changes if needed
- Set reasonable defaults before first detection

**Virtual List Debug Diagnostics:**
```rust
zoon::println!("Container: clientHeight={}, scrollHeight={}", 
    html_el.client_height(), html_el.scroll_height());
```
clientHeight=0 indicates broken Height::fill() chain or layout issues.

## Meta-Pattern: Complex UI Problem Solving

**Systematic approach to complex UI bugs**:

1. **Isolate exact symptom** (e.g., "gap at bottom")
2. **Identify ALL root causes** (not just first one found)
3. **Create incremental fix plan** (testable steps)
4. **Add diagnostic logging** throughout reactive chains
5. **Test each step independently** 
6. **Verify performance** doesn't degrade
7. **Document patterns learned** for future reference

This systematic approach turns complex UI bugs into learning opportunities and reusable patterns.