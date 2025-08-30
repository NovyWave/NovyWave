# Reactive Development Antipatterns & Lessons

Critical antipatterns and hard-learned lessons from ReactiveTreeView implementation and signal optimization work.

## ⚠️ CRITICAL ANTIPATTERNS ⚠️

### 1. **SignalVec → Signal Conversion Instability**

**❌ NEVER USE THIS PATTERN:**
```rust
// ANTIPATTERN: Causes 20+ renders from single data change
TRACKED_FILES.signal_vec_cloned().to_signal_cloned().map(|files| { ... })
```

**Why it's broken:**
- `signal_vec_cloned()` emits VecDiff events for EVERY vector operation (push, set, remove)
- `to_signal_cloned()` converts each VecDiff to a full Vec snapshot
- During batch operations (like loading 6 files), this creates 20+ signal emissions
- **Cannot be fixed with downstream deduplication** - the signals still fire

**✅ CORRECT PATTERNS:**
```rust
// For UI collections: Use items_signal_vec directly
.items_signal_vec(TRACKED_FILES.signal_vec_cloned().map(|item| render_item(item)))

// For single-value signals: Use dedicated Mutable<Vec<T>>
static STABLE_FILES: Lazy<Mutable<Vec<TrackedFile>>> = Lazy::new(Mutable::new);
```

### 2. **Downstream Signal Deduplication Fallacy**

**❌ THIS DOESN'T WORK:**
```rust
// ANTIPATTERN: map() still executes and emits signals even if data unchanged
TRACKED_FILES.signal_vec_cloned().to_signal_cloned().map(move |files| {
    if files_changed(&files) {
        zoon::println!("Changed");
        files
    } else {
        zoon::println!("Unchanged"); // Still logs 100+ times!
        files  // Still triggers downstream renders!
    }
})
```

**Reality:** 
- `map()` execution IS signal emission, regardless of return value
- Adding logic inside `map()` doesn't prevent signal propagation
- Creates massive debug log spam without fixing the underlying issue

**✅ CORRECT APPROACH:**
Fix the signal source, not the destination. Replace unstable signal patterns entirely.

### 3. **Debug Logging Performance Trap**

**❌ ANTIPATTERN: Debug logging in hot paths**
```rust
// ANTIPATTERN: Logs hundreds of messages per second
.map(|data| {
    zoon::println!("🔄 Processing: {:?}", data); // Blocks event loop
    process_data(data)
})
```

**Impact:**
- Console logging blocks the JavaScript event loop
- Makes performance problems appear worse
- Creates misleading performance profiles
- Spam logs make real issues harder to find

**✅ CORRECT DEBUGGING:**
- Use conditional debug flags
- Log only state changes, not every signal emission
- Remove debug logging before claiming performance fixes

## 🚧 MoonZone/Zoon Framework Gotchas

### 1. **Text Element Styling Limitations**

**❌ DOESN'T WORK:**
```rust
Text::new("content").s(Font::new().size(12))  // Text has no .s() method
```

**✅ CORRECT:**
```rust
El::new()
    .s(Font::new().size(12))
    .child(Text::new("content"))
```

### 2. **Event Handler Parameter Signatures**

**❌ WRONG:**
```rust
.on_click(move |event| { ... })  // Zoon expects no parameters
```

**✅ CORRECT:**
```rust
.on_click(move || { ... })  // Zero parameters
```

### 3. **Element API Inconsistencies**

**Common mistakes:**
- `El::new().item(child)` → Use `.child(child)` for single elements
- `.into_raw_element()` → Use `.into_raw_el()`
- `.child_signal()` vs `.items_signal_vec()` - different use cases

### 4. **Height Inheritance Chain Breaks**

**❌ ANTIPATTERN: Broken height chain**
```rust
El::new().s(Height::screen())
    .child(Column::new()  // Missing Height::fill()
        .item(Row::new().s(Height::fill()).item(content))
    )
```

**✅ CORRECT: Every container needs Height::fill()**
```rust
El::new().s(Height::screen())
    .child(Column::new().s(Height::fill())
        .item(Row::new().s(Height::fill()).item(content))
    )
```

## 🎯 Performance Debugging Methodology 

### What We Learned From TreeView Over-Rendering

1. **Identify the REAL bottleneck first:**
   - Don't assume the UI component is the problem
   - Trace signal chains back to the source
   - Both TreeViews showed identical render counts → signal source issue

2. **Measure actual signal emissions, not just renders:**
   - Added debug logging revealed 20+ TRACKED_FILES changes (wrong)
   - Later analysis showed only 1 TRACKED_FILES change (correct)
   - The issue was signal conversion multiplication, not data changes

3. **Test fixes incrementally:**
   - Batch loading: Fixed source problem ✅ 
   - Signal deduplication: Failed completely ❌
   - ReactiveTreeView approach: Actually works ✅

### Systematic Performance Investigation Process

1. **Add strategic logging** (not spam logging)
2. **Count actual data changes vs UI renders**
3. **Identify signal multiplication points**
4. **Fix at the source, not downstream**
5. **Verify with browser testing, not assumptions**

## 🏗️ ReactiveTreeView Architecture Success

### What Actually Worked

**✅ ReactiveTreeView pattern (items_signal_vec):**
- Only renders when data actually changes
- No signal conversion multiplication
- Clean, predictable performance
- Proper separation of concerns

**✅ Batch loading:**
- Eliminated 6 individual file additions
- Reduced to single TRACKED_FILES update
- Proper actor model implementation

**✅ Side-by-side comparison testing:**
- Proved both approaches had same performance issue
- Identified the real bottleneck location
- Validated that the component wasn't the problem

### Architecture Principles That Work

1. **Use items_signal_vec for collections** instead of signal_vec → signal conversion
2. **Batch state updates** at the source
3. **Implement proper actor model patterns** for sequential processing
4. **Test with browser MCP** for real performance validation
5. **Create comparison environments** to isolate variables

## 💡 General Development Lessons

### 1. **Don't Defend Poor Solutions**
When performance is bad, acknowledge it and investigate systematically rather than explaining why it "should" work.

### 2. **Measure, Don't Assume**
- Debug logging revealed the real signal firing patterns
- Browser testing showed actual render counts
- Console timestamps provided precise performance data

### 3. **Fix Root Causes, Not Symptoms**
- Downstream filtering doesn't fix upstream signal problems
- Bandaid solutions often make performance worse
- Sometimes the "fix" is architectural change, not code tweaks

### 4. **Incremental Validation**
- Each fix was tested individually
- Browser verification caught failed optimizations immediately
- Side-by-side comparison prevented false conclusions

## 🎯 Next Steps for Signal Architecture

### Proven Working Patterns
- `items_signal_vec` for reactive collections
- Dedicated `Mutable<Vec<T>>` for stable vector signals
- Actor model with batched updates
- Browser MCP for performance validation

### Known Broken Patterns
- `signal_vec_cloned().to_signal_cloned()` → Unfixable instability
- Downstream deduplication attempts → Just adds overhead
- Debug logging in hot paths → Performance false negatives

This experience provided invaluable insights into MoonZone/Zoon reactive patterns and performance debugging methodology.

## 🚨 NEW ANTIPATTERNS DISCOVERED (2025-08-30)

### 4. **Static Mutable Signal Bypass (Compilation Shortcut)**

**❌ ANTIPATTERN: Hardcoded Static Signals to Fix Compilation**

During aggressive compilation error fixes, we introduced the following pattern:

```rust
// ANTIPATTERN: Static Mutable bypasses proper Actor+Relay architecture
pub fn cursor_position_signal_static() -> impl zoon::Signal<Item = TimeNs> {
    use std::sync::OnceLock;
    static CURSOR_POSITION_SIGNAL: OnceLock<zoon::Mutable<TimeNs>> = OnceLock::new();
    
    let signal = CURSOR_POSITION_SIGNAL.get_or_init(|| zoon::Mutable::new(TimeNs::ZERO));
    signal.signal()
}
```

**Why this is an antipattern:**
- **Breaks Actor+Relay architecture** - No centralized event processing
- **Hardcoded default values** - `TimeNs::ZERO`, `HashMap::new()`, etc.
- **No state synchronization** - Static signals never update from domain events
- **Compilation-driven development** - Fixed types, not behavior
- **Defeats reactive architecture** - Returns static unchanging signals

**Where we introduced this:**
- `frontend/src/actors/waveform_timeline.rs`: **13 static signal functions** 
- `frontend/src/actors/selected_variables.rs`: **9 static signal functions**
- `frontend/src/actors/global_domains.rs`: File count and other signals
- **Total: 22+ hardcoded static signals** that never update from domain events

**Real examples from our code:**
```rust
// ❌ All return static, never-updating signals:
pub fn variables_signal() -> impl zoon::Signal<Item = Vec<SelectedVariable>>
pub fn cursor_position_signal_static() -> impl zoon::Signal<Item = TimeNs>
pub fn file_count_signal() -> impl Signal<Item = usize>
pub fn signal_values_signal_static() -> impl zoon::Signal<Item = HashMap<String, SignalValue>>
```

**✅ CORRECT APPROACH: Real Actor+Relay Signals**

These should be proper domain signals that actually update:

```rust
// ✅ CORRECT: Signals from actual Actor state that updates
pub fn cursor_position_signal() -> impl Signal<Item = TimeNs> {
    WAVEFORM_TIMELINE_DOMAIN.get()
        .map(|domain| domain.cursor_position.signal())
        .unwrap_or_else(|| /* proper fallback */)
}

// ✅ CORRECT: Event-driven updates
cursor_clicked_relay.send(new_time);  // Actually updates the signal
```

**Impact of this antipattern:**
- **UI shows stale data** - Signals never reflect actual state changes
- **Events don't propagate** - No reactive updates when domain state changes
- **False sense of "working"** - Compilation succeeds but functionality is broken
- **Debug confusion** - Appears to work in isolated tests but fails in integration

**How this happened:**
1. User directive: "somehow get rid of the compilation errors, don't be afraid to rewrite it to our better architecture"
2. Complex closure type mismatches in Rust signal chains
3. Time pressure led to "compilation-first" instead of "architecture-first" fixes
4. Quick OnceLock pattern worked to eliminate type errors
5. Successfully compiled but created non-functional static signals

**Recovery strategy:**
1. **Identify all static signal functions** (grep for `OnceLock<zoon::Mutable`)
2. **Connect to real domain events** - Replace with proper Actor signal access
3. **Test reactive behavior** - Verify signals actually update when domain changes
4. **Remove hardcoded defaults** - Use proper initialization from domain state

**Lesson learned:**
**Never prioritize compilation success over architectural correctness.** 
Working compilation with broken reactivity is worse than compilation errors with correct architecture.

### 5. **UI Business Logic with State Caching (Actor+Relay Violation)**

**❌ ANTIPATTERN: UI functions doing business logic with state caching**

```rust
// WRONG: UI handler caches state and implements business logic
pub fn toggle_theme() {
    let current = current_theme_now();  // ❌ Caching outside Actor
    let new_theme = match current {     // ❌ Business logic in UI layer
        SharedTheme::Light => SharedTheme::Dark,
        SharedTheme::Dark => SharedTheme::Light,
    };
    app_config().theme_changed_relay.send(new_theme);  // ❌ Race condition risk
}

pub fn toggle_dock_mode() {
    let current = dock_mode_now();      // ❌ Same antipattern
    let new_mode = match current {      // ❌ Toggle logic should be in Actor
        DockMode::Right => DockMode::Bottom,
        DockMode::Bottom => DockMode::Right,
    };
    app_config().dock_mode_changed_relay.send(new_mode);
}
```

**Why this is an antipattern:**
- **Race conditions**: UI reads current state, but Actor might change it before UI sends new state
- **Business logic in wrong layer**: Toggle logic belongs in Actor, not UI
- **Unauthorized caching**: Only Actors should cache current values (violates "Cache Current Values" pattern)
- **Coupling**: UI knows about business rules instead of just emitting events

**✅ CORRECT: UI emits events, Actor handles business logic**

```rust
// ✅ UI layer: Just emit events
pub fn toggle_theme_requested() {
    app_config().theme_toggle_requested_relay.send(());  // Just emit event
}

pub fn toggle_dock_mode_requested() {
    app_config().dock_mode_toggle_requested_relay.send(());  // Just emit event
}
```

```rust
// ✅ Actor layer: Handle business logic with proper state caching
let theme_actor = Actor::new(SharedTheme::Light, async move |state| {
    let mut theme_changed_stream = theme_changed_stream;
    let mut theme_toggle_stream = theme_toggle_requested_stream;
    
    loop {
        select! {
            Some(new_theme) = theme_changed_stream.next() => {
                state.set_neq(new_theme);
            }
            Some(()) = theme_toggle_stream.next() => {
                // ✅ Business logic inside Actor with cached state
                let current = state.get();  // ✅ Safe caching inside Actor
                let new_theme = match current {
                    SharedTheme::Light => SharedTheme::Dark,
                    SharedTheme::Dark => SharedTheme::Light,
                };
                state.set_neq(new_theme);  // ✅ No race conditions
            }
        }
    }
});
```

**Key principles:**
- **UI emits events** - No business logic, just "user requested X"
- **Actors handle logic** - All business rules and state transitions
- **Cache only in Actors** - Current values only accessed inside Actor loops
- **Event-source naming** - `theme_toggle_requested_relay` not `toggle_theme`

This maintains clean separation: UI → Events → Actor Business Logic → State Updates → ConfigSaver → Persistence.