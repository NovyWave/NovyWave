# Actor+Relay Architecture Patterns

Complete reference for NovyWave's Actor+Relay architecture.

## Core Rules (MANDATORY)

### 1. NO RAW MUTABLES
```rust
// ❌ PROHIBITED
static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();

// ✅ REQUIRED: Domain Actors
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
}

// ✅ REQUIRED: Atom for local UI
let dialog_open = Atom::new(false);
```

### 2. Event-Source Relay Naming
```rust
// ✅ Describe what happened
button_clicked_relay: Relay,
file_loaded_relay: Relay<PathBuf>,

// ❌ Command-like naming
add_file: Relay<PathBuf>,
set_theme: Relay<Theme>,
```

### 3. Domain-Driven Design
```rust
// ✅ Model what it IS
struct TrackedFiles { ... }
struct WaveformTimeline { ... }

// ❌ Enterprise abstractions
struct FileManager { ... }
struct TimelineService { ... }
```

### 4. Cache Current Values (ONLY in Actor loops)
```rust
let actor = ActorVec::new(vec![], async move |state| {
    let mut cached_username = String::new();  // ✅ Cache inside Actor loop
    loop {
        select! {
            Some(username) = username_stream.next() => cached_username = username,
            Some(()) = send_stream.next() => send_message(&cached_username),
        }
    }
});
// ❌ NEVER cache values outside Actor loops
```

### 5. Relay Data Type Constraints
Only simple cloneable data: `Vec<PathBuf>`, `String`, `(String, Result)`, `()`.
Never: `Box<dyn Future>`, `Arc<Connection>`, `Box<dyn FnOnce()>`.

## Domain Patterns

### File Management
```rust
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
    file_selected_relay: Relay<PathBuf>,
    parse_completed_relay: Relay<(String, ParseResult)>,
}

// Usage: Event emission
tracked_files.file_dropped_relay.send(vec![path]);
```

### Variable Selection
```rust
struct SelectedVariables {
    variables: ActorVec<SelectedVariable>,
    variable_clicked_relay: Relay<String>,
    selection_cleared_relay: Relay,
    scope_expanded_relay: Relay<String>,
}
```

## Atom for Local UI State

```rust
struct DialogState {
    is_open: Atom<bool>,
    filter_text: Atom<String>,
    selected_index: Atom<Option<usize>>,
}
```
Use for: dialog visibility, hover effects, search filters, animation states.

## Signal Handler Patterns

```rust
// ✅ Async handlers - naturally break sync chains
COLLECTION.signal_vec_cloned().for_each(move |data| async move {
    send_state_message(Message::ProcessData { data });
}).await;

// ❌ Synchronous handlers cause recursive locks
COLLECTION.signal_vec_cloned().for_each_sync(|data| { ... });
```

## Message Processing

```rust
// ✅ Sequential with yielding
for message in messages {
    Task::next_macro_tick().await;  // ESSENTIAL
    process_message(message).await;
}

// ❌ Concurrent causes races
for message in messages { Task::start(async move { ... }); }
```

## Critical Antipatterns

### 1. No Manager/Service/Controller
Objects manage data, not other objects. Use `TrackedFiles` not `FileManager`.

### 2. Public Field Architecture
```rust
// ✅ Direct public fields
struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,
    pub file_dropped_relay: Relay<Vec<PathBuf>>,
}
tracked_files.file_dropped_relay.send(vec![path]);  // Direct access

// ❌ Helper functions wrapping fields
impl TrackedFiles {
    pub fn send_file_dropped(&self, files: Vec<PathBuf>) { ... }  // Unnecessary
}
```
Default to `pub` fields unless specific reason for privacy.

### 3. zoon::Task Prohibition (CRITICAL)

**NEVER use `zoon::Task::start` - use Actors or `Task::start_droppable` instead**

```rust
// ❌ PROHIBITED: Task::start leaks memory if task runs forever or holds references
zoon::Task::start(async move {
    signal.for_each(|v| { process(v); async {} }).await;
});

// ✅ CORRECT: Actor for event handling
Actor::new((), async move |_| {
    loop { select! { Some(v) = stream.next() => process(v) } }
});

// ✅ CORRECT: Task::start_droppable for bounded one-off operations
let _handle = Task::start_droppable(async move {
    Timer::sleep(5000).await;
    one_time_operation();
});
```

**Rules**:
- **Event handling**: Use Actor, never Task
- **One-off bounded operations**: Use `Task::start_droppable`, store handle if cancellation needed
- **Fire-and-forget bounded ops** (completes within seconds): `Task::start_droppable` acceptable without storing handle

**CRITICAL: TaskHandles MUST be stored to keep tasks alive!**
```rust
// ❌ PROHIBITED: Immediately drops handle, killing the task!
let _ = Task::start_droppable(async move { ... });

// ❌ STILL WRONG: Handle dropped at end of select! branch, killing task!
loop {
    select! {
        Some(file) = file_stream.next() => {
            let _handle = Task::start_droppable(async move {
                Timer::sleep(60_000).await;  // Never reaches here
                timeout_relay.send(file_id);
            });
        }  // _handle dropped here!
    }
}

// ✅ CORRECT: Store handles in collection inside Actor loop
let actor = ActorVec::new(vec![], async move |state| {
    let mut watchdog_handles: Vec<zoon::TaskHandle> = vec![];  // Persists across iterations
    loop {
        select! {
            Some(file) = file_stream.next() => {
                let handle = Task::start_droppable(async move {
                    Timer::sleep(60_000).await;
                    timeout_relay.send(file_id);
                });
                watchdog_handles.push(handle);  // Keep alive!
            }
        }
    }
});

// ✅ CORRECT: Store as struct field for struct lifetime
struct MyDomain {
    _watchdog_task: zoon::TaskHandle,  // Lives with struct
}
```

**Key insight:** `let _handle =` only keeps the handle alive until the end of that scope (e.g., the select! branch).
For long-running watchdogs, collect handles in a Vec that persists across loop iterations.

**Internal Relay Pattern** (eliminates zoon::Task):
```rust
pub struct TrackedFiles {
    pub file_parse_requested_relay: Relay<String>,  // Internal relay
}

// In Actor loop:
parse_requested = file_parse_requested_stream.next() => {
    send_parse_request_to_backend(file_path).await;  // Direct async
}

// Usage: Relay instead of Task
tracked_files.file_parse_requested_relay.send(file_path);  // Not zoon::Task::start()
```

### 4. Data Bundling
```rust
// ❌ Artificial grouping forces unrelated updates
struct PanelDimensions { files_width: f32, timeline_height: f32, ... }

// ✅ Separate actors for independent concerns
struct PanelConfig {
    pub files_panel_width_actor: Actor<f32>,
    pub timeline_panel_height_actor: Actor<f32>,
}
```

### 5. State Access Outside Actors
```rust
// ❌ Race condition: get() + set()
let current = state.get(); state.set(toggle(current));

// ✅ Atomic lock_mut()
{ let mut theme = state.lock_mut(); *theme = match *theme { Light => Dark, Dark => Light }; }
```

### 6. No Temporary Code
No "TODO: implement later" placeholders. Implement properly or don't implement.

## Dependency Injection Patterns

### Parameter Threading vs Context Objects

| Aspect | Parameter Threading | Context Objects |
|--------|-------------------|-----------------|
| Complexity | Simple, direct | Structured |
| Best for | <20 functions | 20+ functions |
| Refactoring | Cascading changes | Isolated to context |

### Parameter Threading
```rust
impl NovyWaveApp {
    pub fn files_panel(&self, app_config: &AppConfig) -> impl Element {
        Column::new()
            .item(self.files_header(app_config))
            .item(self.files_list(app_config))
    }
}
```
Use for: small codebases, simple call chains, stable dependencies.

### Context Objects
```rust
struct TimelineContext {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
}

impl TimelineContext {
    pub fn get_maximum_timeline_range(&self) -> Option<(f64, f64)> {
        let files = self.tracked_files.files_vec_signal.get_cloned();
        // Access via self - no parameter cascading
    }
}
```
Use for: large codebases, complex dependencies, frequent refactoring.

### Clone! Macro
```rust
clone!(variable1, variable2 => move |_| { ... })

// Equivalent to:
{ let variable1 = variable1.clone(); let variable2 = variable2.clone(); move |_| { ... } }
```
Essential for closures in event handlers. Actor/Relay structs designed for cheap cloning.

## Standalone Derived State Actors

For derived data needed by multiple domains:
```rust
pub struct MaximumTimelineRange {
    pub range: Actor<Option<(f64, f64)>>,
    pub range_updated_relay: Relay<Option<(f64, f64)>>,
}
```
- Single source of truth
- Automatic updates when source changes
- Consumers cache in their Actor loops

## Migration Patterns

### Bool → Unit Event
```rust
// ❌ relay::<bool>(); relay.send(true);
// ✅ relay(); relay.send(());
```

### Atom → Actor
Public Atoms allow external `.set()` bypassing domain logic. Use Actor for encapsulation.

### Complex Type Elimination
Replace `Atom<Option<Rc<RefCell<T>>>>` with `Actor<()>` + inline state variables.

## Connection Message Routing (Global Static Elimination)

Replace global MESSAGE_ROUTER/CONFIG_STORE with typed relay subscriptions:

```rust
// ✅ ConnectionMessageActor: Typed relays per message type
pub struct ConnectionMessageActor {
    pub config_loaded_relay: Relay<shared::AppConfig>,
    pub directory_contents_relay: Relay<(String, Vec<Item>)>,
    pub file_loaded_relay: Relay<(String, FileState)>,
    _message_actor: Actor<()>,
}

impl ConnectionMessageActor {
    pub async fn new(mut stream: impl Stream<Item = DownMsg>) -> Self {
        let (config_loaded_relay, _) = relay();
        // ... create relays for each message type

        let message_actor = Actor::new((), async move |_| {
            loop {
                if let Some(msg) = stream.next().await {
                    match msg {
                        DownMsg::ConfigLoaded(c) => config_loaded_relay.send(c),
                        DownMsg::DirectoryContents { path, items } =>
                            directory_contents_relay.send((path, items)),
                        // ... route each type to its relay
                    }
                }
            }
        });
        Self { config_loaded_relay, /* ... */ _message_actor: message_actor }
    }
}

// Domain subscription: subscribe to relevant relays
impl AppConfig {
    pub async fn new(cma: ConnectionMessageActor) -> Self {
        let mut stream = cma.config_loaded_relay.subscribe();
        Actor::new((), async move |_| {
            while let Some(config) = stream.next().await {
                theme_relay.send(config.ui.theme);
            }
        });
    }
}
```

**Benefits:** Type-safe routing, clear dependencies, no global access, reactive compliance.

## Validation Checklist

- [ ] All global Mutables → domain Actors
- [ ] All local Mutables → Atom
- [ ] Event-source relay naming
- [ ] No Manager/Service/Controller
- [ ] Event emission replaces direct mutations
- [ ] Signal-based testing (no .get() methods)
