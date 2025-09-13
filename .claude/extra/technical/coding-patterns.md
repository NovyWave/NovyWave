# Coding Patterns & Best Practices

Core coding patterns, architectural rules, and best practices for NovyWave development.

### State Management Architecture Rules (CRITICAL)

**NEVER use raw Mutables. Follow this hierarchy:**

1. **Atoms for local UI state** - Simple component-level state (hover, focus, local toggles)
2. **Actor+Relay for domain logic** - When Atoms become too limited
3. **Actors instead of Tasks** - For structured async processing with state management

```rust
// ✅ CORRECT: Atoms for simple local state
let is_hovered = Atom::new(false);
let dialog_open = Atom::new(false);

// ✅ CORRECT: Actor+Relay when Atoms are insufficient
struct ToastManager {
    toasts: ActorVec<Toast>,
    toast_dismissed_relay: Relay<String>,
}

// ✅ CORRECT: Actors instead of raw Tasks
let notification_actor = Actor::new(NotificationState::default(), async move |state| {
    // Structured state management with Actor
});

// ❌ WRONG: Raw Mutables
static GLOBAL_STATE: Lazy<Mutable<SomeState>> = Lazy::new(Mutable::new);

// ❌ WRONG: Raw Tasks without state management  
Task::start(async { /* unstructured async work */ });
```

**When to use each:**
- **Atoms**: Button hover, dialog visibility, form inputs, progress bars
- **Actor+Relay**: Cross-component coordination, persistent data, business logic
- **Raw Tasks**: Only for pure utility functions without state

### CRITICAL Architectural Antipatterns

**NEVER create *Manager, *Service, *Controller, or *Handler modules:**

```rust
// ❌ WRONG: Manager modules that don't manage data
struct ToastManager;      // Manages other components, not data
struct DialogManager;     // Artificial abstraction layer
struct StateService;      // Enterprise-style service pattern

// ✅ CORRECT: Model actual domain entities
struct Toasts;           // Collection of toast data
struct DialogState;      // Dialog's actual state
struct AppConfig;        // Configuration data
```

**Reason:** These patterns add unnecessary complexity through indirection. Objects should manage data, not other objects.

### Signal Performance Antipattern (CRITICAL)

**NEVER use `signal_vec().to_signal_vec()` - defeats SignalVec efficiency:**

```rust
// ❌ WRONG: Defeats SignalVec diff efficiency
items.signal_vec().to_signal_vec().map(|items| {
    // Converts efficient diffs back to full Vec snapshots
    render_all_items(items)  // Re-renders everything on any change
})

// ✅ CORRECT: Use SignalVec directly for efficient updates
items.signal_vec().map(|item| {
    render_single_item(item)  // Only re-renders changed items
})
```

**Reason:** SignalVec passes Vec diff changes efficiently. Converting to signal loses this optimization and causes full re-renders.

### Required Imports (dataflow module)

**ALWAYS use dataflow module imports over alternatives:**

```rust
// ✅ CORRECT: Use dataflow imports
use crate::dataflow::{Actor, ActorVec, ActorMap, Relay, Atom};

// ❌ AVOID: Raw zoon/moonzoon alternatives where dataflow exists
use zoon::{Mutable, Task};  // Use Actor+Relay instead
```

### CRITICAL Actor Lifetime Management

**NEVER use underscore prefix for Actors - they get dropped silently:**

```rust
// ❌ WRONG: Actor gets dropped immediately, stops working without errors
let _toast_actor = Actor::new(state, async move |handle| {
    // This will be dropped and never execute!
});

// ✅ CORRECT: Store Actor in element lifecycle
let toast_actor = Actor::new(state, async move |handle| {
    // Timer logic here
});

// Store in element to keep alive
element.after_remove(move |_| drop(toast_actor))
```

**Why underscore prefix is dangerous:**
- Rust allows unused variables with `_` prefix without warnings
- Actors get silently dropped instead of running their async logic
- No compilation errors, but functionality silently breaks
- Especially critical for timer-based Actors that need to stay alive

**General Rule: Don't Use Underscore Prefixes for Unused Variables**
```rust
// ❌ WRONG: Hiding unused variables instead of fixing the issue
if let Some(_unused_value) = some_option {
    // Empty block - variable not actually needed
}

// ❌ ESPECIALLY WRONG: Underscore prefix on Actor parameters - hard to debug!
let actor = Actor::new((), async move |_state| {
    // Actor drops because _state suggests it's unused, but it's NOT!
    _state.set(new_value);  // This line might get overlooked
});

// ✅ CORRECT: Remove unused variables or fix the logic
if some_option.is_some() {
    // If you don't need the value, don't bind it
}

// ✅ CORRECT: Use the variable properly if it's needed
if let Some(value) = some_option {
    process_value(value);  // Actually use it
}

// ✅ CORRECT: Use meaningful names for Actor parameters
let actor = Actor::new((), async move |state_handle| {
    state_handle.set(new_value);  // Clear intent, no confusion
});
}
```

**Why this matters:**
- Underscore prefixes mask design issues instead of fixing them
- Often indicates incomplete implementation or unnecessary code
- Better to either use the variable properly or simplify the logic
- Keeps code clean and intentional

**Proper pattern for local Actors:**
```rust
let timer_actor = Actor::new(TimerState::default(), async move |state| {
    loop {
        Timer::sleep(1000).await;
        // Timer logic
    }
});

// Keep actor alive with element lifecycle
el.after_remove(move |_| drop(timer_actor))
```

### WASM Error Handling Best Practices

**Use WASM-specific error handling methods for better debugging:**

```rust
// ✅ WASM-optimized error handling
use zoon::*; // Provides unwrap_throw, expect_throw

// Better panic messages with expect_throw
let value = option_value
    .expect_throw("Failed to get user configuration - check initialization order");

// Better unwrap with unwrap_throw  
let element = event.target()
    .unwrap_throw(); // Provides proper WASM stack traces

// Use zoon::eprintln! for error logging
zoon::eprintln!("🚨 ERROR: Failed to initialize domain: {}", error_msg);

// Use zoon::println! for normal logging
zoon::println!("✅ Domain initialized successfully");
```

**Why this matters in WASM:**
- `unwrap_throw()` and `expect_throw()` provide proper stack traces in browser dev tools
- `zoon::eprintln!()` goes to console.error() - visible and filterable in browser console  
- `zoon::println!()` goes to console.log() - good for normal logging
- Standard `std::println!()` does nothing in WASM environments
- Better error messages prevent cryptic "unreachable" WASM panics

### Never Swallow Results with `let _ = `

**❌ CRITICAL ANTIPATTERN: Swallowing errors with `let _ = `**
```rust
// WRONG: Silently ignores all errors including network failures, serialization errors, etc.
let _ = CurrentPlatform::send_message(UpMsg::SaveConfig(config)).await;
let _ = function_returning_result();
```

**✅ BETTER: Explicit error handling**
```rust
// Better: Handle or propagate errors properly
if let Err(e) = CurrentPlatform::send_message(UpMsg::SaveConfig(config)).await {
    zoon::eprintln!("🚨 Failed to save config: {e}");
}

// Or if you must ignore but want to see failures in development:
CurrentPlatform::send_message(UpMsg::SaveConfig(config)).await
    .unwrap_or_else(|e| zoon::eprintln!("⚠️ Config save failed: {e}"));

// Or use expect_throw for better WASM debugging:
CurrentPlatform::send_message(UpMsg::SaveConfig(config)).await
    .expect_throw("Critical: Config save must not fail");
```

**Why this matters:**
- `let _ = result` **silently discards all errors** including critical failures
- Network failures, serialization errors, and system issues become invisible
- Makes debugging nearly impossible when things go wrong
- Even `unwrap_throw()` is better because it shows **what** failed and **where**
- Always prefer explicit error handling or at minimum error logging

### Use Zoon Connection.exchange_message for Request-Response

**❌ WRONG: Manual channel-based request-response**
```rust
// Don't implement custom oneshot channels and relay systems
let (sender, receiver) = oneshot::channel::<SharedAppConfig>();
let (config_response_relay, mut config_response_stream) = relay::<SharedAppConfig>();
// Complex manual setup with response tasks and timeouts...
```

**✅ CORRECT: Use Connection.exchange_message**  
```rust
// Zoon Connection provides built-in request-response pattern
let config = connection.exchange_message(UpMsg::LoadConfig).await?;
```

**Key Points:**
- **Zoon Connection has exchange_message method** designed specifically for request-response
- **Examples exist in MoonZoon repo** - always check there first
- **Don't reinvent request-response** - use the framework's built-in solutions
- **Saves complexity** - No manual channels, timeouts, or relay cleanup needed

### Avoid _clone Variable Naming Pattern

**❌ WRONG: Verbose _clone postfix variables**
```rust
let progress_atom_for_actor = progress_atom.clone();
let alert_id_actor = alert_id.clone();
let config_clone = config.clone();
```

**✅ CORRECT: Use clone! macro or block shadowing**
```rust
// Option 1: clone! macro (if available)
clone!(progress_atom, alert_id => async move {
    // Use progress_atom and alert_id directly
});

// Option 2: Block shadowing pattern
{
    let progress_atom = progress_atom.clone();
    let alert_id = alert_id.clone();
    async move {
        // Use clean variable names without _clone suffix
    }
}

// Option 3: Direct shadowing in closure
move || {
    let progress_atom = progress_atom.clone();
    let alert_id = alert_id.clone();
    // Clean names in scope
}
```

**Key Benefits:**
- **Cleaner variable names** - No verbose suffixes
- **Clear ownership transfer** - Explicit about what gets cloned where
- **Readable code** - Variables have their natural names in usage context

### Rust Edition 2024: Explicit Capture Bounds with use<T>

**Understanding `+ use<T>` syntax for fixing lifetime issues:**

```rust
// ✅ CORRECT: Atom.signal() already has proper capture bounds
pub fn signal(&self) -> impl Signal<Item = T> + use<T> {
    self.actor.signal()
}

// This means local atoms should work in signal chains:
let progress_atom = Atom::new(100.0);
.s(Width::percent_signal(progress_atom.signal().map(|p| p as f32)))
```

**What `+ use<T>` does:**
- **Explicit capture bounds** - Only captures the specified generic parameters
- **Prevents overcapturing** - Doesn't automatically capture all lifetimes in scope
- **Enables `'static` usage** - Signal doesn't depend on local lifetimes when not needed
- **Rust Edition 2024 feature** - Gives precise control over `impl Trait` captures

**Why it solves lifetime errors:**
```rust
// OLD: Overcaptures lifetimes, requires 'static
fn old_signal<'a>(data: &'a str) -> impl Signal<Item = String> {
    // Implicitly captures 'a even if not used
}

// NEW: Explicit control over what gets captured  
fn new_signal<'a>(data: &'a str) -> impl Signal<Item = String> + use<> {
    // use<> = capture nothing, works in 'static contexts
}
```

### Modern Rust Formatting Syntax

Use modern Rust formatting macros with inline expressions:

**✅ Modern (Recommended):**
```rust
// Variables directly in format strings
println!("{my_var}");
zoon::println!("{value}");
format!("{name} is {age} years old");

// Debug formatting
println!("{my_var:?}");
zoon::println!("{data:?}");

// Other format specifiers
println!("{value:02}");        // Zero-padded
println!("{value:.2}");        // Decimal places
println!("{value:#x}");        // Hexadecimal
```

**❌ Verbose (Avoid):**
```rust
println!("{}", my_var);
zoon::println!("{}", value);
format!("{} is {} years old", name, age);
```

**Key Benefits:**
- More readable and concise
- Less error-prone (no argument position mismatches)
- Consistent with modern Rust style
- Works with `println!`, `format!`, `zoon::println!`, `eprintln!`, etc.

### WASM Error Logging Best Practice

**✅ ALWAYS use `zoon::eprintln!` for errors with modern formatting:**
```rust
zoon::eprintln!("🚨 APP CONFIG INITIALIZATION FAILED: {error_msg}");
zoon::eprintln!("⚠️ Config save failed: {error}");
zoon::eprintln!("🚨 DOMAIN INITIALIZATION FAILED: {error_msg}");
```

**❌ AVOID: Old verbose formatting for errors:**
```rust
zoon::eprintln!("🚨 Failed: {}", error_msg);  // Verbose, unnecessary
eprintln!("Error: {}", error);                // Wrong function for WASM
```

**Why this matters:**
- `zoon::eprintln!` goes to `console.error()` in browser - properly categorized and filterable
- Modern `{variable}` syntax is cleaner and less error-prone than `{}", variable`
- Standard `eprintln!` does nothing in WASM environments
- Error emojis (🚨⚠️) make errors easily visible in console logs

### CRITICAL: Avoid Logging Large Structs

**❌ NEVER LOG LARGE STRUCTS WITH DEBUG FORMATTING:**
```rust
// ANTIPATTERN: Massive console spam
zoon::println!("File loaded: {:?}", waveform_file);  // 970kb+ of output!
zoon::println!("Config updated: {:?}", entire_config);  // Huge struct dump
```

**✅ CORRECT: Log minimal identifying information:**
```rust
// Only log essential identifying info
zoon::println!("File loaded: {}", waveform_file.id);
zoon::println!("File loaded: {} ({} scopes, {} variables)", 
    file.id, file.scopes.len(), total_variables);
zoon::println!("Config updated: {} fields changed", changed_fields.len());
```

**Why large struct logging is harmful:**
- **Performance**: 970kb+ console output blocks browser rendering
- **Manual debugging**: Massive logs make finding actual issues impossible
- **Automated testing**: Breaks programmatic log parsing and testing tools
- **Development experience**: Console becomes unusable, slows down iteration

**Smart logging alternatives:**
```rust
// Custom Debug implementations for large structs
impl fmt::Debug for WaveformFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WaveformFile({}, {} scopes)", self.id, self.scopes.len())
    }
}

// Selective field logging
zoon::println!("WaveformFile loaded: id={}, format={:?}, scopes={}, time_range={:?}", 
    file.id, file.format, file.scopes.len(), (file.min_time_ns, file.max_time_ns));

// Use conditional debug logging
#[cfg(debug_assertions)]
if VERBOSE_LOGGING.get() {
    zoon::println!("Full struct: {:?}", large_struct);  // Only when explicitly enabled
}
```

### Copy vs Clone for Simple Types

**Prefer `Copy` trait for simple types that should have value semantics:**

```rust
// ✅ CORRECT: Simple enums should derive Copy
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

// ✅ Then use dereference instead of clone
let mut theme = state.lock_mut();
let old_theme = *theme;  // Copy, not clone
*theme = match *theme {  // Direct dereference
    Theme::Light => Theme::Dark,
    Theme::Dark => Theme::Light,
};
```

**❌ AVOID: Using clone() when Copy is available**
```rust
let old_theme = theme.clone();  // Unnecessary allocation
*theme = match &*theme {        // Unnecessary reference
    Theme::Light => Theme::Dark,
    Theme::Dark => Theme::Light,
};
```

**When to use Copy:**
- Simple enums with no data
- Small structs with only Copy fields  
- Numeric types, booleans, chars
- Generally anything ≤ pointer size that should have value semantics

**Benefits of Copy over Clone:**
- No heap allocation
- Cannot fail (no Result return)
- Cleaner syntax with dereference
- Compiler optimizations
- Clear value semantics

### Use Actor Instead of Manual Task Management

**CRITICAL: Always use proper Actor pattern instead of manual TaskHandle management**

**CRITICAL ANTIPATTERN: Task::start for Event Handling**

```rust
// ❌ WRONG: Task::start for event handling anti-pattern
Task::start({
    let is_paused_toggle = is_paused.clone();
    async move {
        while let Some(()) = toast_clicked_stream.next().await {
            let current_paused = is_paused_toggle.signal_ref(|p| *p).to_stream().next().await.unwrap_or(false);
            is_paused_toggle.set(!current_paused);
        }
    }
});

Task::start({
    let dismiss_alert_id = alert_id.clone();
    async move {
        while let Some(()) = dismiss_button_clicked_stream.next().await {
            dismiss_error_alert(&dismiss_alert_id);
        }
    }
});
```

```rust
// ✅ CORRECT: Actor with event stream handling
let toast_actor = Actor::new(ToastState::default(), async move |state| {
    loop {
        select! {
            Some(()) = toast_clicked_stream.next() => {
                // Handle click events with proper state management
                let current_paused = state.lock_ref().is_paused;
                state.lock_mut().is_paused = !current_paused;
            }
            Some(()) = dismiss_button_clicked_stream.next() => {
                dismiss_error_alert(&alert_id);
                break;
            }
            _ = timer_updates => {
                // Handle timer logic
            }
        }
    }
});
```

**Manual Task Anti-Pattern:**
```rust
// ❌ WRONG: Manual task management anti-pattern
#[derive(Debug, Clone)]
struct MyService {
    _task_handle: Arc<TaskHandle>,
}

impl MyService {
    pub fn new() -> Self {
        let task_handle = Task::start_droppable(async move {
            // Service logic
        });
        Self { _task_handle: Arc::new(task_handle) }
    }
}
```

```rust
// ✅ CORRECT: Proper Actor pattern
fn create_my_service_actor() -> Actor<()> {
    Actor::new((), async move |_state| {
        // Service logic with proper Actor lifecycle
    })
}
```

**Why Actor pattern is better:**
- **Centralized event handling** - Single Actor handles all related events
- **State management** - Proper state encapsulation and atomic updates
- **Automatic lifecycle management** - No manual TaskHandle wrappers
- **Consistent architecture** - All state management uses Actor+Relay
- **Cleaner ownership** - No need for `Arc<TaskHandle>` complexity
- **Framework integration** - Actors work seamlessly with signal composition
- **Better debugging** - Actor framework provides better error handling

**When this applies:**
- Event stream handling (clicks, user input, timers)
- Background services (ConfigSaver, network watchers, etc.)
- Signal processing workers
- Any long-running background tasks
- Service-like components that need lifecycle management

**Key Rule: If you're using Task::start to handle event streams, use Actor instead.**

### Avoid Unnecessary Function Indirection

**CRITICAL: Don't create wrapper functions for globals and readonly actors**

```rust
// ❌ UNNECESSARY: Wrapper function adds no value
pub fn current_theme() -> impl Signal<Item = Theme> {
    app_config().theme_actor.signal()  // Just delegates
}

pub fn dock_mode_signal() -> impl Signal<Item = DockMode> {
    app_config().dock_mode_actor.signal()  // Just delegates  
}

// UI calls wrapper
current_theme().map(|theme| render_themed_ui(theme))
```

```rust
// ✅ CORRECT: Direct access to readonly actors
// UI calls directly - no wrapper needed
app_config().theme_actor.signal().map(|theme| render_themed_ui(theme))
app_config().dock_mode_actor.signal().map(|mode| render_dock_mode(mode))
```

**Why indirection is wrong for actors:**
- **Actors are readonly** - No risk of accidental mutations
- **Globals should be accessible** - They exist to be used directly
- **No added safety** - Wrapper doesn't prevent misuse
- **Pure overhead** - Extra function call with zero benefit
- **Code bloat** - More functions to maintain unnecessarily

**When wrapper functions ARE justified:**
- **Complex computation** - `calculate_timeline_position(time, zoom, viewport)`
- **Error handling** - `safe_parse_config(raw_data)` with validation
- **Multiple parameter coordination** - `create_waveform_query(start, end, signals, filters)`

**When wrapper functions are WRONG:**
- **Simple delegation** - `fn get_x() { GLOBAL.x }`  
- **Readonly access** - `fn actor_signal() { ACTOR.signal() }`
- **Global access** - `fn app_state() { &APP_STATE }`
- **Zero logic added** - Just forwarding calls with no benefit

**CRITICAL: Reject common "justifications" for unnecessary wrappers:**

❌ **"API Stability"** - We're inside a Rust app, not a public library. Actors and Relays ARE our API. Let the compiler help with breaking changes instead of hiding them.

❌ **"Future Logic"** - YAGNI (You Aren't Gonna Need It). Adding wrappers "in case we need logic later" creates code bloat. Add logic when you actually need it.

❌ **"Type Simplification"** - Complex signal chains that "need" wrapper functions often indicate smelly code. Fix the underlying complexity instead of hiding it.

**Rust Philosophy: Use the type system, not abstraction layers**
- **Compiler catches breaking changes** - Better than runtime failures
- **Direct actor access** - Cleaner, more explicit code  
- **No premature abstraction** - Add complexity when actually needed
- **Business code ≠ Library code** - Different abstraction needs