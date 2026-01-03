# Technical Reference

Quick reference for NovyWave development patterns and solutions.

## WASM Development

**Logging:** `zoon::println!()` works, `std::println!()` does nothing

**Browser MCP limits:** No F12/F5, no drag/scroll. Use `browser_get_console_logs` and `browser_navigate` instead.

**Build:** Never use `cargo build/check` - only mzoon handles WASM properly. Monitor via `tail -f dev_server.log`.

## Zoon/NovyUI Patterns

```rust
// Icons: enum tokens only
button().left_icon(IconName::Folder)

// Theme-aware colors
.s(Background::new().color_signal(neutral_3().signal()))

// Height inheritance chain - EVERY container needs Height::fill()
El::new().s(Height::screen()).child(Column::new().s(Height::fill())...)

// Type unification in match arms
.item_signal(content.map(|c| match c { A => a().into_element(), B => b().into_element() }))

// TreeView external state
TreeView::new().external_expanded_signal(expanded().map(|s| s))
```

## Configuration System

**New fields require 3 locations:** shared types, SerializableConfig, `load_from_serializable()`

**Initialization order:** `load_config().await` in main before UI starts - eliminates CONFIG_LOADED guards.

## Signal Patterns

**Static vs Reactive:**
```rust
// ❌ Static - never updates
.child_signal(map_ref! { let text = zoon::always(x)...

// ✅ Reactive - updates when data changes
.child_signal(map_ref! { let text = domain_signal()...
```

**Missing dependencies:** Add `_tracked_files` dependency when UI depends on async-loaded data.

**Dedupe for expensive ops:** `signal.dedupe().for_each_sync(|x| expensive(x))`

**Collection to signal:** Use `.signal_vec().to_signal_cloned()` for `map_ref!`

## Debouncing (Actor+Relay Pattern)

```rust
// Nested select! for debouncing - no TaskHandle needed
let actor = Actor::new((), async move |_| {
    loop {
        select! {
            Some(()) = event_stream.next() => {
                loop {
                    select! {
                        Some(()) = event_stream.next() => continue,
                        _ = Timer::sleep(1000) => { save(); break; }
                    }
                }
            }
        }
    }
});
```

## Actor Stream Processing

```rust
// Relay streams are already FusedStream - no .fuse() needed
let mut stream = relay_stream;
loop { select! { Some(x) = stream.next() => ... } }

// Timer::sleep DOES need .fuse() (known futures-rs bug)
_ = Timer::sleep(1000).fuse() => ...
```

## Quick Troubleshooting

### Compilation
- WASM changes not visible: Check `tail -100 dev_server.log | grep -i "error"`
- Only trust mzoon output for WASM status

### Layout
- Height inheritance breaks: Every container needs `Height::fill()`
- TreeView width: container `min-width: max-content`, item `width: 100%`
- Scrolling: Add `min-height: 0` to parent containers

### Reactivity
- UI shows "Loading..." stuck: Check signal chain, `lock_mut().insert()` doesn't trigger signals - use `set_neq()`
- Signal never fires: Check initialization order, missing dependencies
- 30+ renders per change: SignalVec→Signal conversion antipattern - use `items_signal_vec`

### Events/Memory
- Event bubbling: `event.pass_to_parent(false)`
- Canvas coords: `client_x() - canvas_rect.left()`
- Actor dropped early: Store as struct field with `_` prefix to keep alive

### Performance
- Virtual list blanks: Stable element pools, update content only
- Directory scanning: `jwalk::WalkDir` with `parallelism(RayonNewPool(4))`

## Key Debugging Patterns

**Signal routing:** Trace from UI backwards - find what signal UI reads, update THAT signal.

**Drag jump issue:** Sync Actor state with current config-driven UI state when dragging starts.

**Getter antipattern:** Direct public field access reveals Rust 2024 edition `+ use<T>` requirements.

**Loading stuck:** Fast ops appearing slow = broken reactivity, not slow backend.

**Duplicate connections:** Multiple Connection::new() calls cause message routing failures.
