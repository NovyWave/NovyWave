# Development Practices & Workflows

## Following Conventions

When making changes to files, first understand the file's code conventions. Mimic code style, use existing libraries and utilities, and follow existing patterns.

- NEVER assume that a given library is available, even if it is well known. Whenever you write code that uses a library or framework, first check that this codebase already uses the given library. For example, you might look at neighboring files, or check the package.json (or cargo.toml, and so on depending on the language).
- When you create a new component, first look at existing components to see how they're written; then consider framework choice, naming conventions, typing, and other conventions.
- When you edit a piece of code, first look at the code's surrounding context (especially its imports) to understand the code's choice of frameworks and libraries. Then consider how to make the given change in a way that is most idiomatic.
- Always follow security best practices. Never introduce code that exposes or logs secrets and keys. Never commit secrets or keys to the repository.

## Code Style

- IMPORTANT: DO NOT ADD ***ANY*** COMMENTS unless asked
- **NEVER use `#[allow(dead_code)]` or similar warning suppressors** - Remove unused code instead of hiding warnings
  - **EXCEPTION**: Dataflow module APIs (`frontend/src/dataflow/`) can use `#[allow(dead_code)]` for public methods that will be extracted to standalone crate

### Rust Trait Derivation Best Practices (MANDATORY)

**CRITICAL: Prefer derive macros over manual trait implementations for standard traits**

This lesson emerged from complex reactive architecture rewrites where manual implementations added unnecessary complexity and performance issues.

#### Derive Standard Traits Instead of Manual Implementation

**❌ WRONG: Manual Default implementation with "special" values**
```rust
// Custom implementation for simple data structures is verbose and error-prone
impl Default for DragState {
    fn default() -> Self {
        Self {
            active_divider: None,
            drag_start_position: (0.0, 0.0),  // "Special" default values  
            initial_value: 0.0,
        }
    }
}
```

**✅ CORRECT: Clean derived Default**
```rust
// Derive generates optimal implementation automatically
#[derive(Clone, Debug, Default)]
struct DragState {
    active_divider: Option<DividerType>,  // None automatically
    drag_start_position: (f32, f32),      // (0.0, 0.0) automatically
    initial_value: f32,                   // 0.0 automatically  
}
```

#### Add Copy Trait to Eliminate Unnecessary .clone() Calls

**❌ WRONG: Clone-heavy code with small types**
```rust
#[derive(Clone, Debug, PartialEq)]  // Missing Copy
pub enum DividerType {
    FilesPanelMain,
    FilesPanelSecondary,
}

// Results in verbose comparison patterns
matches!(state.active_divider, Some(ref active_type) if *active_type == divider_type)
//                                  ^^^              ^^^^ Dereferencing needed

// And unnecessary .clone() calls
pub fn active_divider_signal(&self) -> impl Signal<Item = Option<DividerType>> {
    self.state.signal_ref(|state| state.active_divider.clone()) // Unnecessary allocation
}
```

**✅ CORRECT: Copy trait eliminates allocations**
```rust
#[derive(Clone, Copy, Debug, PartialEq)]  // Added Copy
pub enum DividerType {
    FilesPanelMain,
    FilesPanelSecondary,
}

// Clean comparison without references
matches!(state.active_divider, Some(active_type) if active_type == divider_type)

// Automatic copying without allocation  
pub fn active_divider_signal(&self) -> impl Signal<Item = Option<DividerType>> {
    self.state.signal_ref(|state| state.active_divider) // Copy automatically
}
```

#### When to Use Copy vs Clone

**Use Copy for:**
- Enums with simple variants (no heap data)
- Small structs (primitives, small fixed arrays)
- Types that should be passed by value efficiently

**Use Clone only for:**
- Types containing heap-allocated data (String, Vec, etc.)
- Large structures where copying is expensive
- Types that need custom cloning behavior

#### Standard Trait Derivation Guidelines

**Always derive when possible:**
```rust
// Standard derives for most types
#[derive(Debug, Clone, PartialEq)]        // Basic derives
#[derive(Debug, Clone, Copy, PartialEq)]  // + Copy for small types
#[derive(Debug, Clone, Default)]          // + Default for initializable types
```

**Benefits:**
- **Performance**: Derived implementations are optimized by the compiler
- **Correctness**: Less chance of manual implementation errors  
- **Maintenance**: Automatically updated when fields change
- **Consistency**: Standard behavior across the codebase

**Key Lesson:** During complex architectural transformations, prefer Rust idioms (derive macros, Copy semantics) over manual implementations to reduce cognitive overhead and eliminate performance issues.

### Modern Rust Formatting Syntax (MANDATORY)

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

### Compilation Error Verification (CRITICAL)

**MANDATORY: Always verify ALL compilation errors are actually resolved before claiming success**

- **NEVER report success without verification**: Always check `tail -100 dev_server.log | grep -E "error\[E[0-9]+\]" | wc -l` returns 0
- **Warnings vs Errors**: Only warnings are acceptable - even 1 compilation error means task is incomplete
- **Real Example**: Reported "32 errors fixed" when 1 error remained due to missing `.await` on async function call
- **Verification Commands**:
  ```bash
  # Count remaining errors (must be 0)
  tail -100 dev_server.log | grep -E "error\[E[0-9]+\]" | wc -l
  
  # Show error details if any exist
  tail -100 dev_server.log | grep -A 5 "error\[E"
  ```

**Why This Matters**: Incomplete error resolution breaks compilation and wastes debugging time later.

### Business Logic Preservation During Refactoring (CRITICAL)

**MANDATORY: Never remove business logic during architectural refactoring - always preserve and convert**

**❌ WRONG: Removing business logic to fix compilation errors**
```rust
// ANTIPATTERN: Comment out complex logic during refactoring
// pub async fn compute_maximum_timeline_range(&self) -> Option<(f64, f64)> {
//     // TODO: Restore timeline range computation logic later
// }

// ANTIPATTERN: Placeholder implementations that discard business logic
pub fn get_variables_from_tracked_files(scope_id: &str) -> Vec<VariableWithContext> {
    Vec::new()  // TODO: Implement proper variable filtering later
}
```

**✅ CORRECT: Convert business logic to new architecture while preserving functionality**
```rust
// ✅ GOOD: Convert sync logic to async signal-based patterns
pub async fn compute_maximum_timeline_range(&self) -> Option<(f64, f64)> {
    let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
    let loaded_files: Vec<shared::WaveformFile> = tracked_files
        .iter()
        .filter_map(|tracked_file| match &tracked_file.state {
            shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
            _ => None,
        })
        .collect();
    
    let selected_file_paths = self.get_selected_variable_file_paths().await;
    // ... preserve all original business logic, just converted to new patterns
}
```

**Key Principles:**
- **Convert, don't delete** - Transform existing logic to new architecture patterns
- **Preserve all business rules** - Timeline calculations, filtering logic, validation must remain
- **No "implement later" TODOs** - If the logic was working before, make it work in the new architecture
- **Test equivalent behavior** - Converted logic should produce the same results

**Why this matters:**
- **Prevents functionality regression** - Users expect existing features to keep working
- **Avoids context loss** - Business logic represents domain knowledge that's hard to recreate
- **Maintains code quality** - No partially implemented functions in the codebase

### Expressive State Types (CRITICAL)

**MANDATORY: Never use misleading default values - use expressive types that represent actual state**

**❌ WRONG: Misleading defaults that look like real data**
```rust
// DANGER: 0 or (0,0) looks like valid data but represents "no data"
let canvas_width = 0.0;         // Is this "no canvas" or "zero-width canvas"?
let timeline_range = (0.0, 0.0); // Is this "no range" or "zero-duration range"?
let cursor_position = 0;        // Is this "no position" or "start position"?
```

**✅ CORRECT: Expressive types that clearly represent state**
```rust
// Clear state representation with Option
let canvas_width: Option<f32> = None;           // Clearly "not measured yet"
let timeline_range: Option<(f64, f64)> = None;  // Clearly "no data loaded"
let cursor_position: Option<TimeNs> = None;     // Clearly "no position set"

// Even better: Custom enums for complex states
#[derive(Debug, Clone)]
enum CanvasState {
    NotMeasured,
    Measuring,
    Ready { width: f32, height: f32 },
    ResizeInProgress { old: (f32, f32), new: (f32, f32) },
}

#[derive(Debug, Clone)]
enum TimelineRange {
    NoData,
    Loading,
    Ready { start: f64, end: f64 },
    Error(String),
}

// Advanced: Nested options for complex scenarios
type DataState<T> = Result<Option<T>, String>; // Error | Loading(None) | Ready(Some(data))
```

**Key Principles:**
- **Never use 0, (0,0), empty strings as "no data" indicators**
- **Use `Option<T>` for simple present/absent states**
- **Use custom enums when multiple states exist (Loading, Error, etc.)**
- **Consider `Result<Option<T>, E>` for error + loading + data scenarios**
- **Make invalid states unrepresentable through types**

**Why this matters:**
- **Debugging nightmare prevention**: No more guessing "is 0 a real value or no data?"
- **Type safety**: Compiler forces handling of all possible states
- **Self-documenting**: Code clearly expresses what state represents
- **Prevents silent failures**: Can't accidentally use "no data" as real data

### Comment Antipatterns to Avoid

**NEVER add unnecessary code comments** like:
- `// ✅ CLEANED UP` - Code should be self-evident 
- `// removed` - Version control handles this
- `// migrated` - Don't document refactoring history in code
- `// eliminated` - Obvious from absence of code
- `// consolidated` - Implementation speaks for itself

**NEVER add verbose structural comments** like:
- `// ===== SIGNAL ACCESS =====` - Section dividers add noise
- `// Get signal for all tracked files` - Function name is self-documenting
- `// Sync dedicated Vec signal after ActorVec change` - Obvious from implementation
- `// Found files list (debug info omitted for performance)` - Unnecessary annotations
- `// Actor+Relay compliance explanation` - Belongs in documentation, not code
- `// Implementation details for obvious patterns` - Code structure is clear

**Systematic Comment Cleanup Success Pattern:**
```rust
// ❌ BEFORE: Verbose, redundant commentary (370 lines)
// ===== PUBLIC API FUNCTIONS =====

/// Get signal for all tracked files - provides reactive access
/// to the complete collection of tracked files with proper
/// Actor+Relay architecture compliance
pub fn files_signal(&self) -> impl Signal<Item = Vec<TrackedFile>> {
    // Sync the dedicated Vec signal to ensure consistency
    // This maintains proper Actor+Relay patterns
    self.files_vec_signal.signal_cloned()
}

// ✅ AFTER: Clean, focused code (261 lines)
pub fn files_signal(&self) -> impl Signal<Item = Vec<TrackedFile>> {
    self.files_vec_signal.signal_cloned()
}
```

**What to Preserve (no explicit request needed):**
- **Essential module/file documentation**: `//! Module purpose and architectural context`
- **Non-obvious business logic context**: `// Timeline uses nanosecond precision for VCD compatibility`
- **Complex algorithm explanations**: `// Binary search with interpolation between transition points`
- **Public API documentation**: `/// Returns reactive signal that updates when files change`
- **Domain-specific technical details**: When implementation choices aren't obvious from naming

**Results of Proper Comment Cleanup:**
- 30% reduction in file size typical
- Significantly improved readability
- Zero impact on functionality
- Code self-documents through clear structure

**Principle:** Clean code documents itself through clear naming and structure. Comments should only explain *why* non-obvious decisions were made, never *what* the code obviously does.

### CRITICAL: Analyze Code Context Before Removing TODOs

**MANDATORY: Always examine what the code actually DOES before removing any TODO comments.**

**The Broken Functionality Trap:**
```rust
// ❌ DANGEROUS: Removing TODO without analyzing the code
let _selected_vars = variables_signal => {
    if *files_count == 0 {
        None
    } else {
        None  // ← This always returns None! TODO was marking broken functionality
    }
}
```

**TODO Classification Protocol:**
1. **Read the code the TODO points to** - Don't just read the TODO text
2. **Identify TODO type**:
   - **Architectural TODOs**: Nice-to-have improvements → Can remove if obvious from code
   - **Functionality TODOs**: Point to broken/missing features users expect → Must keep

**Examples of Functionality TODOs (NEVER remove):**
```rust
Text::new("-- ns/px")  // TODO: Connect to zoom level signal
Text::new("--s")       // TODO: Connect to cursor position signal
El::new()              // TODO: Implement format dropdown
```

**Examples of Architectural TODOs (safe to remove if obvious):**
```rust
// TODO: Refactor to use better signal pattern (when current code works)
// TODO: Consider performance optimization (when feature is functional)
```

**Key Rule:** If removing a TODO would hide actually broken functionality that users expect to work, the TODO must stay.

### UI Hardcoded Values Exception

**Design system tokens are preferred** for UI spacing, colors, and dimensions, but **hardcoded values are acceptable within UI functions for locality**:

```rust
// ✅ ACCEPTABLE: Hardcoded values within UI function for locality
fn dropdown_component() -> impl Element {
    let item_height = 28.0;  // Local to this component
    let border_width = 1.0;  // Component-specific styling
    
    Column::new()
        .s(Height::exact(item_height as u32))
        .s(Borders::all(Border::new().width(border_width as u32)))
}

// ✅ PREFERRED: Design system tokens when available
fn button_component() -> impl Element {
    Button::new()
        .s(Padding::all(SPACING_12))
        .s(Gap::new().x(SPACING_8))
}
```

**Key principle**: UI locality trumps global consistency - hardcoded values within UI functions are acceptable when they improve code readability and component cohesion.

## Dataflow API Protection

**CRITICAL: Do not modify the dataflow module API without explicit confirmation**

- **NEVER modify Actor/Relay API** - The `pub state` field will NOT be part of the public API
- **Ask for confirmation** before any changes to `frontend/src/dataflow/` module
- **Preserve API boundaries** - Internal implementation changes only, no public interface modifications  
- **Use existing patterns** - Work within current Actor+Relay constraints rather than extending API

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
- **CRITICAL: NO cargo check EVER** - Use `tail -50 dev_server.log | grep "warning:"` for warnings instead
- **NEVER restart dev server** without permission - compilation takes minutes
- Monitor: `makers start > dev_server.log 2>&1 &`
- Check: `tail -f dev_server.log` for build status
- Use: `makers kill` and `makers start` commands only

### WASM I/O Constraints (CRITICAL)
- **MANDATORY: All I/O operations must be performed on the backend side**
- **NEVER use synchronous filesystem operations in WASM**: `std::fs::read_dir()`, `std::fs::File::open()` block the main thread
- **Main thread blocking causes browser tab freezes**: Requires force-kill to recover
- **Proper pattern**: Frontend sends `UpMsg` to backend → Backend performs I/O → Backend responds with `DownMsg`
- **Example**: File browsing uses `UpMsg::BrowseDirectory` → backend filesystem operations → `DownMsg::DirectoryContents`
- **Why this matters**: WASM runs on browser main thread - any blocking operation freezes the entire tab

### WASM Error Handling & Logging

**Quick Reference:**
```rust
// ✅ WASM-specific methods (provide proper stack traces)
value.expect_throw("descriptive error message");  // Better than expect()
value.unwrap_throw();                              // Better than unwrap()
zoon::println!("info");   // console.log()
zoon::eprintln!("error"); // console.error()
// Note: std::println!() does nothing in WASM

// ❌ Never silently swallow errors
let _ = result;  // Silent failure - use if let Err(e) or expect_throw

// ✅ Use Connection.exchange_message for request-response (not manual channels)
let config = connection.exchange_message(UpMsg::LoadConfig).await?;
```

**Never log large structs**: `{:?}` on WaveformFile = 970kb+ output blocking browser. Log only IDs and counts.

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

> **📖 Complete Reference:** See `.claude/extra/technical/reactive-antipatterns.md` for comprehensive reactive patterns, antipatterns, debugging methodology, and testing patterns.

**Quick Checklist:**
- Use `map_ref!` for combining signals; add `_tracked_files` for file dependencies
- Use `saturating_sub()` for counts; `.into_element()` for type unification
- Never let derived signals modify source data (causes infinite loops)
- Check console for 30+ renders in <300ms (over-rendering symptom)

**See system.md for task management, git workflows, and subagent delegation.**

## Architectural Migration Best Practices

### Clean Slate Transformation Rule

**CRITICAL: For major architectural migrations, never create bridge patterns or parallel APIs**

When transforming architectures (e.g., global state → self-contained patterns), follow these principles:

```rust
// ❌ WRONG: Bridge/adapter patterns create schizophrenic architecture
// During global → ChatApp migration:
struct GlobalBridge {
    global_files: &'static TrackedFiles,      // Old system
    app_files: &'self TrackedFiles,           // New system
}

// ❌ WRONG: Parallel API maintenance
pub fn files_panel_global() -> impl Element { ... }  // Old API
pub fn files_panel_self(&self) -> impl Element { ... } // New API

// ❌ WRONG: Compatibility layers
trait FilesPanelCompat {
    fn render() -> impl Element;
}
```

**✅ CORRECT: Clean slate approach**
```rust
// 1. Create backup files for safety
// cp views.rs views.rs.backup
// cp global_domains.rs global_domains.rs.backup

// 2. Complete transformation - no halfway measures
struct NovyWaveApp {
    tracked_files: TrackedFiles,    // New architecture only
    selected_variables: SelectedVariables,
}

impl NovyWaveApp {
    fn files_panel(&self) -> impl Element { ... }  // Single API
}

// 3. Delete old architecture files entirely
// rm global_domains.rs dialog_manager.rs error_manager.rs
```

**Migration Strategy:**
- **Clean slate migration** - no bridges, adapters, or parallel APIs
- **Create backup files** of source code before starting transformation  
- **All-or-nothing conversion** - commit to new architecture completely
- **Single validation** after complete conversion only

**Why this matters:**
- **Prevents architectural confusion** - developers know which pattern to follow
- **Eliminates maintenance burden** - no dual codepaths to maintain
- **Forces commitment** - can't fall back to old patterns
- **Cleaner end result** - no legacy code or compatibility layers

## CRITICAL: Never Hardcode Dynamic Values

**MANDATORY RULE: Never hardcode any values that should be dynamic - you'll forget it and then debugging will be hell**

### Quick Reference

```rust
// ❌ PROHIBITED: Any hardcoded values in data processing
value: "a".to_string(),           // Hardcoded formatted output
let canvas_width = 800.0;         // Hardcoded dimension
let timeout_ms = 5000;            // Hardcoded constant

// ✅ CORRECT: Use actual data sources
let value = waveform_signal.to_bit_string();  // Dynamic from real data
let canvas_width = current_canvas_width();    // From DOM/signals
let timeout_ms = app_config().network_timeout_ms;  // From config
```

### Prevention Strategy

1. **Search for suspicious patterns**: `rg '"[^"]*"\.to_string\(\)'` in data code
2. **Question every value**: Is this computed or just hardcoded?
3. **Mark test data**: Always add `TODO: Replace with real data`
4. **Trace data flow**: Follow values from UI back to source

**Exception**: Debug-only hardcodes with explicit TODO comments explaining why and when to remove.

**Impact**: Hardcoded values cause debugging misdirection - spending hours in frontend when issue is trivial backend mock data.

## Work Integrity & Problem-Solving Ethics

### Check for Existing Functional Code First (CRITICAL)

**MANDATORY: Always look for existing working code before implementing new features or debugging**

- **Integration over reinvention** - Existing working systems often just need proper connection
- **Architecture archaeology** - Search codebase for similar functionality that may already exist
- **Backend-first assumption** - Check if backend implementation exists before building frontend from scratch
- **Connection over creation** - Often the solution is connecting to working code, not writing new code

**Real-World Success Example (Load Files Dialog Fix):**
```rust
// ❌ WRONG: Reimplementing filesystem operations in WASM
std::fs::read_dir(path) // Causes browser freeze

// ✅ CORRECT: Connecting to existing working backend
connection.send_up_msg(UpMsg::BrowseDirectory { path }).await;
// Backend already had working directory browsing - just needed connection!
```

**Key Questions to Ask:**
- **Does this functionality already exist somewhere?**
- **Is there a working backend implementation I should connect to?**
- **Am I reinventing something that already works?**
- **What existing patterns can I extend rather than replace?**

**User Guidance Pattern:** *"backend implementation for getting directories was perfectly working before, your job was to connect it with already partly functioning dialog"*

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

### Avoid Ugly Hacks That Circumvent Best Practices

**CRITICAL PRINCIPLE: Never implement ugly hacks that work around Rust best practices or established patterns/antipatterns**

- **Don't bypass lifetime rules** with unsafe code or artificial static lifetimes when proper ownership patterns exist
- **Don't circumvent borrow checker** with excessive cloning, RefCell overuse, or pointer tricks when Actor+Relay patterns provide the solution
- **Don't work around type system** with `as` casting, `transmute`, or `Any` trait abuse when proper type design is available
- **Don't hack around compilation errors** with deprecated functions, temporary bridges, or architectural violations when the error reveals a real design issue

**✅ CORRECT: Fix the root architectural issue**
```rust
// ✅ GOOD: Proper Actor+Relay pattern
let actor = Actor::new(State::default(), async move |state| {
    // Clean, idiomatic Rust following established patterns
});

// ✅ GOOD: Proper signal-based reactive updates  
.child_signal(domain_actor.signal().map(|state| render_state(state)))
```

**❌ WRONG: Ugly hacks to work around proper patterns**
```rust
// ❌ BAD: Static lifetime hack to avoid proper ownership
static GLOBAL_HACK: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

// ❌ BAD: Unsafe pointer hack to bypass borrow checker
let ptr = &state as *const State as *mut State;
unsafe { (*ptr).modify() }  // Violates Rust safety

// ❌ BAD: Arc<Mutex<>> overuse to avoid proper Actor patterns
let shared_hack = Arc::new(Mutex::new(ComplexState::default()));
```

**Key Principle:** When you encounter resistance from Rust's type system or compilation errors, this usually indicates a design issue that should be solved architecturally, not hacked around technically.

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
- **NEVER create generic files** like `utils.rs`, `helpers.rs`, `types.rs`, or `state.rs` with common business-related helpers

### Avoid Generic Utility Files

**CRITICAL PRINCIPLE: Business logic belongs in domain-specific modules, not generic containers**

**❌ PROHIBITED: Generic business utility files**
```rust
// ❌ BAD: Generic dumping grounds
utils.rs          // Becomes dumping ground for miscellaneous functions
helpers.rs        // Vague responsibility, grows into architectural graveyard
state.rs          // Central state becomes migration comment repository
types.rs          // Type definitions divorced from their domain context
variable_helpers.rs  // Business logic divorced from domain
```

**✅ CORRECT: Domain-specific placement**
```rust
// ✅ GOOD: Business logic lives with its domain
selected_variables.rs   // Variable utilities IN the variables domain
tracked_files.rs       // File utilities IN the files domain
error_display.rs       // Error utilities IN the error domain
config.rs             // Config utilities IN the config domain
```

**Why generic files are harmful:**
- **Architectural graveyards** - Accumulate migration comments and deprecated code
- **Unclear ownership** - No clear responsibility for maintenance and evolution
- **Import confusion** - Developers unsure where to find or place business logic
- **Coupling issues** - Generic files often import from multiple domains
- **Testing difficulty** - Mixed responsibilities make focused testing harder

**Instead:** Place utility functions directly in their domain modules where they belong.

### File Splitting by Domain Objects (MANDATORY)

**CRITICAL: When splitting large files, organize by domain objects, not by item types**

**✅ CORRECT: Split by domain objects/responsibilities**
```rust
// Split timeline_actor.rs into domain-driven modules:
timeline/
  waveform_timeline.rs     // WaveformTimeline actor and its business logic
  maximum_range.rs         // MaximumTimelineRange standalone actor  
  cursor_control.rs        // CursorController actor and its operations
  viewport_manager.rs      // ViewportManager actor and its state
```

**❌ WRONG: Split by technical categories (types, utils, helpers)**
```rust  
// DON'T create technical utility modules:
timeline/
  types.rs                 // Generic type definitions divorced from domain
  events.rs               // Event handlers separated from their actors
  utils.rs                // Utility functions divorced from domain logic
  cache.rs                // Cache operations separated from their domain
```

**Key Principle:** Each module should contain **complete domain responsibility** - the actor, its types, its operations, and its business logic together, not scattered across technical utility files.

**Domain Object Organization Benefits:**
- **Complete ownership** - Each module owns its complete domain
- **Single responsibility** - Clear boundaries between domain concerns  
- **Easy navigation** - Find all timeline cursor logic in cursor_control.rs
- **Natural testing** - Test complete domain behaviors, not scattered utilities
- **Clear dependencies** - Domain interactions are explicit, not hidden in utils

### Successful Modularization Example: timeline_actors.rs

**PROVEN SUCCESS PATTERN:** The timeline_actors.rs modularization achieved 57% size reduction using domain extraction:

**Original:** 1,593 line monolithic file with 6+ Actor systems  
**Result:** Clean domain-driven modules following MaximumTimelineRange extraction pattern

```rust
// ✅ EXTRACTED DOMAIN CONTROLLERS (908 lines total):
timeline/
  maximum_timeline_range.rs    // Derived state actor for timeline bounds
  timeline_cache.rs           // Signal data caching (169 lines)
  cursor_animation.rs         // Cursor movement animation (140 lines)  
  panning_controller.rs       // Left/right viewport panning (95 lines)
  canvas_state.rs            // Canvas dimensions and rendering (195 lines)
  zoom_controller.rs          // Complete zoom management (282 lines)
```

**Extraction Benefits Achieved:**
- ✅ **Domain-driven separation** - Each controller manages one functional area
- ✅ **Clean boundaries** - Minimal coupling between controllers
- ✅ **Actor+Relay compliance** - No architectural violations
- ✅ **Maintainable size** - Each module 100-300 lines vs 1,593 line monolith
- ✅ **Complete responsibility** - Controllers own their complete domain logic

**Replication Pattern:** Use this same approach for other monolithic files by identifying distinct Actor systems and extracting them as complete domain controllers following the MaximumTimelineRange extraction pattern.

### Autonomous Sustained Work Pattern

For extended autonomous work: Create comprehensive TodoWrite lists (40+ items), mark completed immediately, use subagents for parallel analysis, test fixes incrementally, never claim completion without verification. Work systematically: investigate → fix → test → verify → mark complete.

### Planning Guidelines

**When user uses "PLAN" keyword**: Present comprehensive plan using TodoWrite before making ANY changes. Wait for approval before implementation.

**Tool Usage**: Use Task tool in plan mode. Only use exit_plan_mode for implementation planning, NOT for research tasks.

## Systematic Multi-Session Task Methodology

**Use for:** Architecture migrations, multi-session refactors, large-scale cleanups (10+ files), tasks with high regression risk.

### 5-Phase Safety Pattern

1. **Preparation**: Read antipattern docs, review patterns, check current system state, create TodoWrite plan
2. **Implementation**: Single focus area, 10-15 changes max, file-by-file, verify after each change
3. **Verification**: Check for new antipatterns, confirm compilation, test critical functionality
4. **Documentation**: Update docs with resolved issues, commit with descriptive messages
5. **Monitoring**: Weekly health checks, review commits for antipattern introduction

### Key Principles

- **Never skip verification** after each step
- **Revert immediately** if error/warning count increases or new antipatterns appear
- **Single focus area** per session - never multiple architectural domains simultaneously
- **Commit between sessions** with session results documented
- If methodology failing (chaos, regressions): Return to Phase 1 with smaller scope

