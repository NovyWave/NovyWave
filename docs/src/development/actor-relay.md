# Actor+Relay Pattern

NovyWave uses a custom Actor+Relay architecture for reactive state management, designed specifically for Rust/WASM applications.

## Core Concepts

### Actors
Actors encapsulate state and process events sequentially:

```rust
let counter = Actor::new(0, async move |state| {
    loop {
        select! {
            Some(()) = increment_stream.next() => {
                let current = state.get();
                state.set(current + 1);
            }
        }
    }
});
```

### Relays
Relays are typed event channels with event-source naming:

```rust
// Event-source naming: describes what happened
let (button_clicked_relay, button_clicked_stream) = relay();

// Send events
button_clicked_relay.send(());
```

### Signals
Actors expose reactive signals for UI binding:

```rust
// Read-only signal access
.child_signal(counter.signal().map(|count| {
    Text::new(format!("Count: {}", count))
}))
```

### Atoms
Atoms handle simple local UI state:

```rust
let hover_state = Atom::new(false);

.on_hovered_change(move |hovered| hover_state.set(hovered))
```

## Why Actor+Relay?

### Problems with Raw Mutables

```rust
// ❌ Anti-pattern: Raw global mutables
static FILES: Lazy<MutableVec<File>> = Lazy::new(MutableVec::new);

// Issues:
// - No encapsulation
// - Race conditions possible
// - Hard to trace state changes
```

### Actor+Relay Solution

```rust
// ✅ Proper Actor+Relay pattern
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
}

// Benefits:
// - Encapsulated state
// - Sequential event processing
// - Clear event flow
```

## Event-Source Naming

Relay names describe **what happened**, not what to do:

```rust
// ✅ Good: Event-source naming
button_clicked_relay: Relay,
file_loaded_relay: Relay<PathBuf>,
input_changed_relay: Relay<String>,

// ❌ Bad: Command-style naming
add_file: Relay<PathBuf>,
set_theme: Relay<Theme>,
```

## Cache Current Values Pattern

When an Actor needs multiple values to process an event, cache them inside the Actor loop:

```rust
let actor = Actor::new(state, async move |state| {
    // Cache values inside loop
    let mut cached_username = String::new();
    let mut cached_message = String::new();

    loop {
        select! {
            Some(username) = username_stream.next() => {
                cached_username = username;
            }
            Some(message) = message_stream.next() => {
                cached_message = message;
            }
            Some(()) = send_stream.next() => {
                // Use cached values
                send_message(&cached_username, &cached_message);
            }
        }
    }
});
```

**Critical:** Only cache inside Actor loops, never in UI components.

## Domain Modeling

Model **what things are**, not what manages them:

```rust
// ✅ Good: Domain-driven naming
struct TrackedFiles { ... }
struct SelectedVariables { ... }
struct WaveformTimeline { ... }

// ❌ Bad: Enterprise abstractions
struct FileManager { ... }
struct VariableService { ... }
```

## UI Integration

### Reading State

```rust
// Use signals for UI binding
.child_signal(domain.actor.signal().map(|state| {
    render_state(state)
}))
```

### Emitting Events

```rust
// Emit events through relays
.on_click(move || {
    domain.button_clicked_relay.send(());
})
```

### Collections

```rust
// Use items_signal_vec for efficient collection rendering
Column::new()
    .items_signal_vec(
        domain.items.signal_vec().map(|item| render_item(item))
    )
```

## Best Practices

1. **No raw Mutables** - Use Actor+Relay or Atom
2. **Event-source naming** - Describe events, not commands
3. **Domain-driven design** - Model what things are
4. **Cache in Actors only** - Never cache in UI
5. **Public fields** - Use public fields, not getters
6. **No Manager/Service** - Avoid enterprise patterns

## Further Reading

- See `frontend/src/dataflow/` for implementation
- See `docs/actors_relays/` for detailed examples
