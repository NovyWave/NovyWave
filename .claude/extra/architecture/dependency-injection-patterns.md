# Dependency Injection Patterns: Parameter Threading vs Context Objects

**CRITICAL ARCHITECTURAL CHOICE**: When eliminating global state, choose between parameter threading and context object patterns based on scale, complexity, and maintainability needs.

## Pattern Comparison Overview

| Aspect | Parameter Threading | Context Objects |
|--------|-------------------|-----------------|
| **Complexity** | Simple, direct | Structured, object-oriented |
| **Maintainability** | Good for small projects | Better for large projects |
| **Type Safety** | Individual parameters | Grouped dependencies |
| **Refactoring** | Cascading changes | Isolated to context |
| **Performance** | Minimal overhead | Small cloning overhead |

## Parameter Threading Pattern

**Definition**: Adding domain parameters directly to function signatures throughout the call chain.

### Implementation Example from NovyWave APP_CONFIG Elimination

```rust
// ✅ PARAMETER THREADING: Direct parameter passing
impl NovyWaveApp {
    pub fn files_panel(&self, app_config: &AppConfig) -> impl Element {
        Column::new()
            .item(self.files_header(app_config))
            .item(self.files_list(app_config))
            .item(self.files_footer(app_config))
    }
    
    fn files_header(&self, app_config: &AppConfig) -> impl Element {
        Row::new()
            .item(self.title_text())
            .item(self.dock_toggle_button(app_config))
            .item(self.theme_button(app_config))
    }
    
    fn dock_toggle_button(&self, app_config: &AppConfig) -> impl Element {
        button()
            .label_signal(app_config.dock_mode_actor.signal().map(|mode| {
                match *mode {
                    shared::DockMode::Right => "Dock Bottom",
                    shared::DockMode::Bottom => "Dock Right",
                }
            }))
            .on_press(clone!(app_config => move |_| {
                app_config.dock_mode_toggle_requested_relay.send(());
            }))
    }
}

// Usage: Thread parameters through call chain
impl Element for NovyWaveApp {
    fn into_raw_element(self) -> RawElement {
        let app_config = crate::config::app_config();  // Get once at top level
        self.main_layout(&app_config)  // Thread through all calls
    }
}
```

### Benefits of Parameter Threading

1. **Explicit Dependencies**: Clear what each function needs
2. **Type Safety**: Compiler enforces parameter requirements
3. **Simple Pattern**: Easy to understand and implement
4. **Minimal Abstraction**: Direct access without wrapper objects
5. **Compile-Time Checking**: Missing parameters cause compilation errors

### Drawbacks of Parameter Threading

1. **Cascading Changes**: Adding new dependency requires updating entire call chain
2. **Function Signature Bloat**: Functions can accumulate many parameters
3. **Maintenance Overhead**: Large refactors touch many function signatures
4. **Repetitive Code**: Same parameters passed through multiple levels

## Context Object Pattern

**Definition**: Using `self` as dependency context with domain references, eliminating parameter cascading.

### Implementation Example

```rust
// ✅ CONTEXT OBJECT: Self as dependency context
struct UIContext {
    pub tracked_files: TrackedFiles,
    pub selected_variables: SelectedVariables,
    pub waveform_timeline: WaveformTimeline,
    pub app_config: AppConfig,
}

impl UIContext {
    pub fn new(
        tracked_files: TrackedFiles,
        selected_variables: SelectedVariables,
        waveform_timeline: WaveformTimeline,
        app_config: AppConfig,
    ) -> Self {
        Self { tracked_files, selected_variables, waveform_timeline, app_config }
    }
    
    /// Domain access through self - no parameter cascading
    pub fn files_panel(&self) -> impl Element {
        Column::new()
            .item(self.files_header())        // No parameters needed
            .item(self.files_list())          // Self contains everything
            .item(self.files_footer())
    }
    
    pub fn files_header(&self) -> impl Element {
        Row::new()
            .item(self.title_text())
            .item(self.dock_toggle_button())  // Accesses self.app_config internally
            .item(self.theme_button())
    }
    
    pub fn dock_toggle_button(&self) -> impl Element {
        button()
            .label_signal(self.app_config.dock_mode_actor.signal().map(|mode| {
                match *mode {
                    shared::DockMode::Right => "Dock Bottom",
                    shared::DockMode::Bottom => "Dock Right",
                }
            }))
            .on_press(clone!(self.app_config => move |_| {
                app_config.dock_mode_toggle_requested_relay.send(());
            }))
    }
    
    /// Complex operations with multiple domains via self
    pub fn get_maximum_timeline_range(&self) -> Option<(f64, f64)> {
        let files = self.tracked_files.files_vec_signal.get_cloned();
        let selected_vars = self.selected_variables.variables_vec_signal.get_cloned();
        // Use both domains through self
        compute_timeline_range(&files, &selected_vars)
    }
}

// Usage: Create context once, use methods everywhere
let ui_context = UIContext::new(tracked_files, selected_variables, waveform_timeline, app_config);
let main_panel = ui_context.files_panel();  // Clean method call
let timeline_range = ui_context.get_maximum_timeline_range();
```

### Benefits of Context Objects

1. **No Parameter Cascading**: Dependencies injected once at context creation
2. **Clean Method Signatures**: Methods only need self
3. **Grouped Responsibilities**: Related functionality naturally organized
4. **Easy Refactoring**: Adding new domain only affects context struct
5. **Dependency Clarity**: All dependencies declared in context struct
6. **Self as Context**: Natural Rust pattern using self for state access

### Drawbacks of Context Objects

1. **Additional Abstraction**: Extra layer between functions and dependencies
2. **Context Design**: Need to decide what belongs in each context
3. **Cloning Overhead**: Context objects need to be cloneable for some uses
4. **Larger Structs**: Context objects can accumulate many fields

## Specific Examples from NovyWave Global Elimination

### Global State Access (Before)
```rust
// ❌ BEFORE: Global static access everywhere
fn dock_toggle_button() -> impl Element {
    button()
        .on_press(|| {
            let current_mode = crate::config::app_config().dock_mode_actor.get();
            // Direct global access
        })
}

fn theme_button() -> impl Element {
    button()
        .on_press(|| {
            let current_theme = crate::config::app_config().theme_actor.get();
            // Direct global access
        })
}
```

### Parameter Threading Approach (After Global Elimination)
```rust
// ✅ PARAMETER THREADING: Explicit dependencies
impl NovyWaveApp {
    pub fn toolbar(&self, app_config: &AppConfig) -> impl Element {
        Row::new()
            .item(self.dock_toggle_button(app_config))    // Pass parameter
            .item(self.theme_button(app_config))          // Pass parameter
            .item(self.load_files_button(app_config))     // Pass parameter
    }
    
    fn dock_toggle_button(&self, app_config: &AppConfig) -> impl Element {
        button().on_press(clone!(app_config => move |_| {
            app_config.dock_mode_toggle_requested_relay.send(());
        }))
    }
    
    fn theme_button(&self, app_config: &AppConfig) -> impl Element {
        button().on_press(clone!(app_config => move |_| {
            app_config.theme_toggle_requested_relay.send(());
        }))
    }
}
```

### Context Object Approach (Alternative)
```rust
// ✅ CONTEXT OBJECT: Self contains all dependencies
struct AppContext {
    app_config: AppConfig,
    tracked_files: TrackedFiles,
    // Other domains...
}

impl AppContext {
    pub fn toolbar(&self) -> impl Element {
        Row::new()
            .item(self.dock_toggle_button())     // No parameters!
            .item(self.theme_button())           // No parameters!
            .item(self.load_files_button())      // No parameters!
    }
    
    fn dock_toggle_button(&self) -> impl Element {
        button().on_press(clone!(self.app_config => move |_| {
            app_config.dock_mode_toggle_requested_relay.send(());
        }))
    }
    
    fn theme_button(&self) -> impl Element {
        button().on_press(clone!(self.app_config => move |_| {
            app_config.theme_toggle_requested_relay.send(());
        }))
    }
}
```

## Decision Matrix

### Choose Parameter Threading When:
- ✅ **Small to medium codebase** (< 20 utility functions)
- ✅ **Clear function boundaries** - each function has specific purpose
- ✅ **Stable dependencies** - domain requirements don't change frequently
- ✅ **Simple call chains** - not many levels of function nesting
- ✅ **Type safety priority** - want explicit compile-time dependency checking

### Choose Context Objects When:
- ✅ **Large codebase** (20+ utility functions with shared dependencies)
- ✅ **Complex dependency webs** - many functions need multiple domains
- ✅ **Frequent refactoring** - dependencies change or new domains added
- ✅ **Related functionality grouping** - natural clusters of operations
- ✅ **Maintenance priority** - easier long-term evolution over compile-time checks

## Hybrid Approach

**Best of Both Worlds**: Use context objects for complex areas, parameter threading for simple ones.

```rust
// Complex UI rendering - use context objects
struct UIContext { /* many domains */ }
impl UIContext {
    pub fn complex_panel(&self) -> impl Element { /* uses self.domain */ }
}

// Simple utility functions - use parameter threading  
impl NovyWaveApp {
    fn simple_button(&self, app_config: &AppConfig) -> impl Element {
        // Single dependency, simple function
    }
}
```

## Migration Strategy

### From Global State to Local Dependencies

1. **Identify Access Patterns**: Map where global state is used
2. **Choose Pattern Based on Complexity**: Parameter threading for simple, context objects for complex
3. **Create Context Structs** (if using context pattern): Group related domains
4. **Thread Dependencies**: Add parameters to function signatures
5. **Apply Clone! Macro**: Handle lifetime issues in closures
6. **Test Incrementally**: Ensure functionality preserved
7. **Optimize**: Remove unused parameters or context fields

## Key Insights from NovyWave Experience

**The parameter threading approach was successful for NovyWave because:**
- Relatively focused UI codebase with clear function boundaries
- Most functions only needed 1-2 domain dependencies
- Clone! macro eliminated lifetime complexity
- Explicit dependencies improved code clarity

**Context objects would be better for:**
- Larger applications with 50+ UI functions
- Complex domain interactions requiring multiple dependencies
- Frequent architectural changes requiring dependency evolution

## Pattern Rule

**Start with parameter threading for simplicity, migrate to context objects when parameter cascading becomes unwieldy (typically around 20+ functions or 5+ domains per function).**

Both patterns are valid architectural choices - the key is choosing based on project scale, complexity, and maintenance requirements rather than personal preference.