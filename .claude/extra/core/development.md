# Development Practices & Workflows

## Following Conventions

When making changes to files, first understand the file's code conventions. Mimic code style, use existing libraries and utilities, and follow existing patterns.

- NEVER assume that a given library is available, even if it is well known. Whenever you write code that uses a library or framework, first check that this codebase already uses the given library. For example, you might look at neighboring files, or check the package.json (or cargo.toml, and so on depending on the language).
- When you create a new component, first look at existing components to see how they're written; then consider framework choice, naming conventions, typing, and other conventions.
- When you edit a piece of code, first look at the code's surrounding context (especially its imports) to understand the code's choice of frameworks and libraries. Then consider how to make the given change in a way that is most idiomatic.
- Always follow security best practices. Never introduce code that exposes or logs secrets and keys. Never commit secrets or keys to the repository.

## Code Style

- IMPORTANT: DO NOT ADD ***ANY*** COMMENTS unless asked

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

// ✅ CORRECT: Remove unused variables or fix the logic
if some_option.is_some() {
    // If you don't need the value, don't bind it
}

// ✅ CORRECT: Use the variable properly if it's needed
if let Some(value) = some_option {
    process_value(value);  // Actually use it
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

## Refactoring Rules

**ATOMIC CODE MOVEMENT - NEVER BREAK COMPILATION:**
1. Copy complete code blocks to destination files first
2. Verify compilation succeeds after each copy
3. Only then remove code from source files
4. NEVER create placeholder functions or empty stubs
5. NEVER rename types with aliases (e.g., `Signal as DataSignal`) - move code directly
6. Always preserve exact functionality during moves

## State Management: Actor+Relay Architecture (MANDATORY)

> **📖 Complete Actor+Relay Reference:** See @.claude/extra/architecture/actor-relay-patterns.md for comprehensive patterns, examples, and implementation details.

**CRITICAL RULES (Quick Reference):**
- **NO raw Mutables** - Use Actor+Relay for domain logic, Atom for local UI state
- **Event-source relay naming** - `button_clicked_relay` not `add_file_relay`
- **Domain-driven design** - `TrackedFiles` not `FileManager`
- **NO Manager/Service/Controller** - Objects manage data, not other objects

## Mandatory Clarification Protocol

**CRITICAL: Always ask clarifying questions before starting complex tasks.**

### When to Ask Questions:
- Multi-step UI/UX tasks requiring specific styling or behavior
- Tasks with ambiguous requirements or multiple interpretation paths
- Complex technical implementations where assumptions could lead to rework
- Integration work where exact behavior needs specification

### Question Types to Ask:
- **Specificity**: "Should the entire file extension part '(*.vcd, *.fst)' be less contrasting, or just the asterisks?"
- **Context**: "Do you want the dialog to take full screen height, or screen height with padding?"
- **Scope**: "Should scrollbar styling match Files & Scope panels specifically?"
- **Verification**: "Can you clarify what you mean by 'background doesn't cover content'?"

### Example Good Clarification:
```
I have a couple of clarifying questions before I start implementing:

1. **Window title styling**: For "Select Waveform Files (*.vcd, *.fst)" - should the entire file extension part "(*.vcd, *.fst)" be less contrasting, or just the asterisks and dots?

2. **Full screen height**: When you say "fill parent height (effectively fill screen)" - do you want the dialog to take up the full viewport height with some margin, or literally edge-to-edge?

3. **Background coverage**: Can you clarify what you mean by "Dialog background does not cover entire content"?

Once I understand these details clearly, I'll implement all the improvements efficiently.
```

**Never assume - always clarify ambiguous requirements upfront.**

## Development Server Management

### Server Management Rules
- **ABSOLUTE PROHIBITION: NEVER run dev server or compilation commands yourself**
- **DO NOT** execute `makers start`, `makers kill`, `makers build`, or any compilation commands
- **DO NOT** attempt to manage the mzoon dev server process  
- **ALWAYS** read `dev_server.log` to check compilation status - this is everything you need
- If auto-compilation appears to not be working, **TELL THE DEVELOPER** to start the mzoon CLI
- Backend/shared crate compilation takes DOZENS OF SECONDS TO MINUTES - this is normal
- **WAIT ENFORCEMENT: Must wait for compilation to complete, no matter how long**
- **NEVER use `cargo build/check`** - Only mzoon handles WASM properly
- **NEVER restart dev server** without permission - compilation takes minutes
- Monitor: `makers start > dev_server.log 2>&1 &`
- Check: `tail -f dev_server.log` for build status
- Use: `makers kill` and `makers start` commands only

### Log Monitoring Patterns
```bash
# Check for compilation errors
tail -100 dev_server.log | grep -i "error"

# Monitor for successful compilation
tail -f dev_server.log | grep -i "compilation complete"

# Debug patterns
rg "println!" --type rust  # Find debug statements to clean up
```

## Testing & Verification Protocols

### CRITICAL VERIFICATION REQUIREMENTS
- **NEVER claim success without actual verification**
- **ALWAYS use browser MCP for visual verification of UI changes**
- **ALWAYS check compilation logs for errors before testing**
- If you CANNOT verify a fix (compilation fails, browser unreachable, etc.) - **TELL THE USER IMMEDIATELY**
- Never claim "it works" or "it's fixed" without actual testing

### UI Testing Protocol
```bash
# 1. Check compilation status first
tail -f dev_server.log

# 2. Verify frontend compilation succeeds
# Look for "Frontend compilation complete" or similar

# 3. Use browser MCP to test changes
mcp__browsermcp__browser_navigate "http://localhost:8080"
mcp__browsermcp__browser_screenshot  # Full page or element-specific screenshots
```

**Screenshot Options:**
- Full page: `mcp__browsermcp__browser_screenshot` (captures entire page)
- Element-specific: Can target specific elements using CSS selectors to save tokens and focus on relevant UI areas

### Example Honest Responses
- "I cannot verify the fix works because compilation is failing"
- "Browser shows the dialog is still not centered - the fix didn't work"
- "I see scrollbar errors in the console - the styling isn't applying"

### Three-Stage Testing Approach
1. **Compilation Verification**: Ensure code builds without errors
2. **Visual Verification**: Use browser MCP to test UI changes
3. **Functional Verification**: Test actual behavior matches requirements

## Reactive Code Development & Review

### Reactive Code Review Checklist

**Before Writing Reactive Code:**
- [ ] Identify all signals that should trigger updates
- [ ] Check for potential circular dependencies (A→B→A)
- [ ] Plan initialization order (config load → state setup → UI render)
- [ ] Consider state preservation during updates

**Signal Chain Design:**
- [ ] Use `map_ref!` for combining multiple signals
- [ ] Add `_tracked_files` pattern for file loading dependencies
- [ ] Convert `SignalVec` with `.to_signal_cloned()` before use in `map_ref!`
- [ ] Use `.into_element()` for type unification in conditional signals

**Common Pitfall Prevention:**
- [ ] Derived signals don't modify their source data
- [ ] Use `saturating_sub()` instead of `-` for count calculations  
- [ ] Dynamic UI elements use `label_signal` or `child_signal` patterns
- [ ] One-shot initialization preserves existing states
- [ ] Compare values before updating (`if current != new`)

### Step-by-Step Reactive Debugging

**1. Identify the Issue Type:**
```bash
# Check console for infinite loop patterns
grep -i "rendering\|computing\|processing" console.log | wc -l
# If >1000 lines, likely infinite loop
```

**2. Trace Signal Dependencies:**
```rust
// Add temporary debug logging
zoon::println!("🔍 Signal {} triggered", signal_name);
```

**3. Check for Common Issues:**
- Missing signal dependencies (UI doesn't update)
- Circular signal chains (infinite loops)  
- Bidirectional reactive flows (config ↔ state)
- Integer overflow in calculations
- Signal type mismatches (`Clone` errors)

**4. Apply Systematic Fixes:**
- Add missing dependencies to signal chains
- Break circular dependencies with one-shot patterns
- Use state preservation during updates
- Convert signal types properly
- Test incrementally after each fix

### Over-Rendering Recognition
**Symptoms:** 30+ identical render logs in <300ms, UI flickering, browser lag
**Common pattern:** `TRACKED_FILES → SMART_LABELS → child_signal(map_ref!)`
**Fix approach:** Remove intermediate signals, direct computation, add signal deduplication

### Reactive Antipatterns to Avoid

**❌ Circular Signal Dependencies:**
```rust
// Bad: Derived signal modifies its source
SMART_LABELS.signal().for_each_sync(|labels| {
    TRACKED_FILES.lock_mut().update_labels(labels); // Triggers SMART_LABELS again!
});
```

**❌ Missing Signal Dependencies:**
```rust  
// Bad: Variables panel won't update when files load
map_ref! {
    let scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()) =>
    get_variables_from_files(&scope_id) // Missing file loading dependency
}
```

**❌ Static State in Dynamic UI:**
```rust
// Bad: Checkbox always unchecked regardless of selection
CheckboxBuilder::new().checked(false)
```

**❌ Bidirectional Reactive Flow:**
```rust
// Bad: Config and state both trigger each other
config_changes.for_each_sync(|config| update_state(config));
state_changes.for_each_sync(|state| update_config(state)); // Creates loop!
```

### Reactive Testing Patterns

**Manual Signal Testing:**
```rust
#[cfg(debug_assertions)]
static SIGNAL_FIRE_COUNT: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

// In signal chain:
let count = SIGNAL_FIRE_COUNT.get() + 1;
SIGNAL_FIRE_COUNT.set(count);
if count > 100 {
    zoon::println!("⚠️ POTENTIAL INFINITE LOOP: {} fires", count);
}
```

**Integration Testing:**
```rust
// Test initialization doesn't cause loops
async fn test_config_load_stability() {
    initialize_from_config().await;
    
    // Wait for signals to stabilize
    Timer::sleep(1000).await;
    
    // Check no excessive signal firing occurred
    assert!(SIGNAL_FIRE_COUNT.get() < 50);
}
```

**See `.claude/extra/technical/reactive-patterns.md` for comprehensive patterns and examples.**

**See system.md for complete task management protocols.**

**See system.md for git workflows and safety rules.**


**See system.md for complete subagent delegation strategies.**

## CRITICAL: Never Hardcode Dynamic Values

**MANDATORY RULE: Never hardcode any values that should be dynamic - you'll forget it and then debugging will be hell**

### The Hardcoded Mock Data Nightmare

**Real Example from NovyWave:** Signal formatting appeared completely broken across the entire frontend - all format options (Bin, Hex, Oct, etc.) showed wrong values. Hours of debugging frontend formatting logic, signal chains, and UI components revealed the real issue was hardcoded mock data in the backend:

```rust
// ❌ DISASTER: Hardcoded formatted values instead of raw data
SignalTransition {
    time_ns: 0,
    value: "a".to_string(),        // Should be "1010" (binary) not "a" (formatted hex)
},
SignalTransition {
    time_ns: 50_000_000_000,
    value: "3".to_string(),        // Should be "11" (binary) not "3" (formatted decimal)
},
SignalTransition {
    time_ns: 0,
    value: "c".to_string(),        // Should be "1100" (binary) not "c" (formatted hex)
},
```

### Why This Is Catastrophic

1. **Debugging Misdirection**: Spend hours debugging complex frontend logic when the issue is trivial backend mock data
2. **False Architectural Problems**: Assume signal formatting systems are broken when they work correctly
3. **Wasted Development Time**: Multiple attempted fixes in wrong codebase areas
4. **User Frustration**: Broken functionality with no apparent cause

### Prevention Rules

**✅ CORRECT: Dynamic data or clearly marked test data**
```rust
// ✅ GOOD: Use actual waveform parsing
let value = waveform_signal.to_bit_string();  // Dynamic from real data

// ✅ GOOD: If must use test data, make it obvious and correct
SignalTransition {
    time_ns: 0,
    value: "1010".to_string(),    // ✅ Raw binary that frontend expects
    // TODO: Replace with actual waveform parsing
},
```

**❌ NEVER: Hidden hardcoded values**
```rust
// ❌ EVIL: Looks like real data but is hardcoded formatted output
value: format_signal_for_display(&signal),  // Hardcoded result, not dynamic
value: "calculated_result".to_string(),     // Fake "calculated" result
value: some_complex_function_that_returns_hardcoded_value(), // Hidden hardcoding
```

### Enforcement Strategy

1. **Search for hardcoded strings**: `rg '"[^"]*"\.to_string\(\)'` in data processing code
2. **Question every "example" value**: Is this actually computed or just hardcoded?
3. **Mark temporary test data**: Always add `TODO: Replace with real data` comments
4. **Trace data flow**: Follow values from UI back to source - are they actually dynamic?

**Remember: Hardcoded values that look dynamic are debugging time bombs that will waste hours of your life.**

### CRITICAL: NEVER Use Numeric Hardcoded Values

**❌ ABSOLUTELY PROHIBITED: Numeric constants in business logic**
```rust
// ❌ DISASTER: Hardcoded numeric values create maintenance nightmares
let stable_viewport_range_ns = 250_000_000_000_u64; // Hardcoded 250 seconds
let default_canvas_width = 800.0; // Hardcoded canvas size
let timeout_ms = 5000; // Hardcoded timeout

// ❌ Even with "good" variable names, still hardcoded
let TIMELINE_DURATION_NS = 250_000_000_000_u64; // Still hardcoded!
```

**✅ CORRECT: Use proper data sources**
```rust
// ✅ Get actual viewport range from Actor state
let viewport_range_ns = viewport_actor.signal().map(|v| v.end.nanos() - v.start.nanos());

// ✅ Get actual canvas dimensions from DOM/signals
let canvas_width = current_canvas_width();

// ✅ Get timeouts from configuration
let timeout_ms = app_config().network_timeout_ms;
```

**ONLY Exception: Debug/temporary fixes with explicit TODO**
```rust
// TODO: REMOVE DEBUG HARDCODED VALUE once viewport actor signal access is fixed
// CRITICAL: This 250s hardcode prevents viewport corruption during resize,
// but must be replaced with proper viewport_actor.signal() access
let stable_viewport_range_ns = 250_000_000_000_u64; // 250 seconds - DEBUG ONLY
```

**Why numeric hardcoding is catastrophic:**
- **Data changes**: Real timelines aren't always 250s - FST files can be microseconds to hours
- **Configuration drift**: Hardcoded values become stale when configs change
- **Testing issues**: Unit tests with different data sizes break with hardcoded assumptions
- **Maintenance hell**: Finding and updating scattered numeric constants across codebase

### Conditional Logging Antipattern

**❌ CRITICAL ANTIPATTERN: Hardcoded conditional logging thresholds**

```rust
// ❌ DISASTER: Hardcoded magic numbers in logging conditions
if width > 520.0 || height > 100.0 {
    zoon::println!("🔧 CANVAS: Resized to {}x{} px", width, height);
}

// ❌ Future debugging nightmare: What are 520.0 and 100.0?
// - Are these canvas sizes? Screen dimensions? Arbitrary thresholds?
// - Will break when debugging smaller screens or different layouts
// - No context for why these specific numbers were chosen
```

**Why this is catastrophic:**
- **Debugging blindness**: Silent failures when values fall below arbitrary thresholds
- **Future developer confusion**: No context for why these specific numbers exist
- **Layout dependency**: Breaks when UI layout changes (responsive design, different screen sizes)
- **False debugging assumptions**: Developer assumes logging covers all cases

**✅ CORRECT approaches:**
```rust
// Option 1: Log everything (if performance allows)
zoon::println!("🔧 CANVAS: Resized to {}x{} px", width, height);

// Option 2: Conditional logging with clear semantic meaning
const CANVAS_MIN_LOGGABLE_SIZE: f32 = 100.0; // Document why this threshold exists
if width >= CANVAS_MIN_LOGGABLE_SIZE {
    zoon::println!("🔧 CANVAS: Resized to {}x{} px", width, height);
}

// Option 3: Remove logging entirely if it's not essential
// (Often the best choice for frequent events like resize)
```

**Key principle:** If logging is causing performance issues, reduce by removing entire log categories, not by adding mysterious conditional thresholds.

## Work Integrity & Problem-Solving Ethics

### No Shortcuts or Paper-Over Solutions

**CRITICAL PRINCIPLE: Either do the work properly or be honest about limitations**

- **Never create shortcuts** that paper over architectural problems without solving them
- **Never add deprecation warnings** as a substitute for actual fixes
- **Never claim to have "fixed" something** when you've only hidden the problem
- **Be honest** when a task is too difficult, complex, or requires knowledge/tools you don't have
- **Admit limitations** rather than delivering incomplete or cosmetic solutions

**Examples of prohibited shortcuts:**
```rust
// ❌ SHORTCUT: Adding deprecated escape hatches instead of proper architecture
#[deprecated(note = "Use signal() instead")]
pub fn current_value(&self) -> T {
    self.state.get_cloned()  // Still breaks architecture!
}

// ✅ PROPER: Implement actual Actor+Relay patterns or be honest about complexity
// "This requires implementing proper caching inside Actor loops using the 
// 'Cache Current Values' pattern, which is a significant architectural change
// that needs careful analysis of all usage sites."
```

**Honest responses when work is too complex:**
- "This requires a complete migration to reactive patterns across 15+ call sites"
- "I don't have the tools to analyze all the reactive dependencies properly"  
- "This architectural change needs careful design - let me break it into smaller steps"
- "The serialization patterns need Actor-internal caching which is complex to implement correctly"

**Quality over appearance:** Better to deliver partial but correct work than complete but broken work.

## Quality Assurance & Best Practices

### Debug Cleanup Methodology
1. Use Task tool subagents for parallel analysis
2. Categorize warnings: definitely fixable vs maybe fixable vs keep as-is
3. Remove dead code first
4. Add TODO comments + `#[allow]` for future features
5. Challenge clone variables - test compilation without them
6. Achieve 100% warning cleanup for production-ready codebase

### Error Handling Verification
- Always use `error_display::add_error_alert()` for ALL error handling
- Never duplicate logging
- Test error states with invalid inputs
- Verify graceful degradation

### Important Development Reminders
- Do what has been asked; nothing more, nothing less
- NEVER create files unless they're absolutely necessary
- ALWAYS prefer editing an existing file to creating a new one
- NEVER proactively create documentation files (*.md) or README files unless explicitly requested

### Autonomous Sustained Work Pattern

When users request extended autonomous work (e.g. "I won't be here, work as long as possible"), use this proven pattern for sustained productivity:

**Setup for Extended Sessions:**
1. **Create comprehensive todo lists** - Break complex problems into 40+ specific actionable todos
2. **Use TodoWrite proactively** - Track all progress in real-time, mark completed immediately 
3. **Systematic problem-solving** - Use subagents for parallel analysis and implementation
4. **Continuous progress validation** - Test fixes incrementally, never claim completion without verification

**Key Success Factors:**
- **Detailed planning prevents getting stuck** - Comprehensive todos provide clear next steps
- **Real-time progress tracking** - TodoWrite keeps work organized and prevents losing focus
- **Subagent delegation** - Extends effective working time by using separate context spaces
- **Systematic approach** - Complete one issue fully before moving to next

**Example Pattern from Successful Signal Loop Fix:**
```
1. Create 40+ todos covering: root cause analysis, systematic fixes, testing, verification
2. Work through each systematically: investigate → fix → test → verify → mark complete
3. Use subagents for: codebase analysis, pattern searching, comprehensive audits
4. Continuous testing: browser console monitoring, compilation verification, UI testing
5. Result: 14+ reactive antipatterns eliminated over extended session
```

This pattern enables sustained autonomous work while maintaining quality and preventing getting lost in complex problems.

### Planning Guidelines
- Use the Task tool when you are in plan mode
- Only use exit_plan_mode tool when planning implementation steps for code writing tasks
- For research tasks (gathering information, searching, reading), do NOT use exit_plan_mode

