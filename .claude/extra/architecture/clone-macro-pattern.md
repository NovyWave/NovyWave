# Clone! Macro Pattern for Rust Lifetime Resolution

**CRITICAL ARCHITECTURAL PATTERN**: The `clone!` macro solves complex Rust lifetime issues in reactive/async contexts, particularly when eliminating global state access.

## Pattern Definition

The `clone!` macro creates local clones of variables for use in closures and async contexts, eliminating lifetime and borrowing issues during architectural migrations.

### Syntax

```rust
clone!(variable1, variable2, variable3 => move |parameters| {
    // Use variable1, variable2, variable3 without lifetime issues
})

// Equivalent to:
{
    let variable1 = variable1.clone();
    let variable2 = variable2.clone();
    let variable3 = variable3.clone();
    move |parameters| {
        // Use cloned variables
    }
}
```

## Real-World Examples from NovyWave Global Elimination

### Before: Complex Manual Cloning
```rust
// ❌ BEFORE: Complex manual cloning for each closure
.on_press({
    let app_config_for_press = app_config.clone();
    let tracked_files_for_press = tracked_files.clone();
    let selected_variables_for_press = selected_variables.clone();
    move || {
        request_load_waveform_files(
            &app_config_for_press,
            &tracked_files_for_press, 
            &selected_variables_for_press
        );
    }
})
```

### After: Clean Clone! Macro Usage
```rust
// ✅ AFTER: Clean clone! macro
.on_press(clone!(app_config, tracked_files, selected_variables => move |_| {
    request_load_waveform_files(&app_config, &tracked_files, &selected_variables);
}))
```

### Signal Handler Context
```rust
// ✅ Complex signal chains with multiple dependencies
.child_signal(clone!(app_config, tracked_files => move |_| {
    map_ref! {
        let opened_files = tracked_files.files.signal_vec_cloned().to_signal_cloned(),
        let dock_mode = app_config.dock_mode_actor.signal() => {
            render_files_panel(opened_files, dock_mode)
        }
    }
}))

// Without clone! macro, this would require:
// let app_config_for_signal = app_config.clone();
// let tracked_files_for_signal = tracked_files.clone();
// .child_signal(move |_| { ... })
```

### Async Task Context
```rust
// ✅ Async task with multiple domain dependencies
zoon::Task::start(clone!(app_config, connection, tracked_files => async move {
    while let Some(down_msg) = down_msg_stream.next().await {
        match down_msg {
            DownMsg::WaveformFile { file_path, waveform_file } => {
                tracked_files.parse_completed_relay.send((file_path, Ok(waveform_file)));
                save_workspace_config(&app_config);
            }
            DownMsg::LoadingError { file_path, error } => {
                tracked_files.parse_error_relay.send((file_path, error));
            }
        }
    }
}));
```

## When to Use Clone! Macro

### Primary Use Cases

1. **Global State Elimination**: When removing global static access and threading dependencies
2. **Reactive Signal Chains**: Multiple dependencies in `map_ref!` or `child_signal` contexts
3. **Event Handlers**: Button clicks, input changes with multiple domain dependencies
4. **Async Tasks**: Background processing with domain state access
5. **Closure Lifetime Issues**: When manual cloning becomes verbose and error-prone

### Performance Considerations

**✅ Acceptable Cloning Scenarios:**
```rust
// Actor/Relay structures are designed for cheap cloning
struct AppConfig { ... }  // Arc-wrapped internally
struct TrackedFiles { ... }  // ActorVec clones are cheap
struct SelectedVariables { ... }  // Domain objects optimized for cloning
```

**⚠️ Avoid for Heavy Data:**
```rust
// Don't use clone! for actual data payloads
let large_file_contents = vec![0u8; 1_000_000];
// Don't: clone!(large_file_contents => move |_| { ... })
// Do: Pass by reference or use Arc<T> explicitly
```

## Benefits

1. **Lifetime Simplification**: Eliminates complex Rust borrowing issues
2. **Code Clarity**: Clear dependency declaration at closure site
3. **Reduced Boilerplate**: Single line vs multiple clone statements
4. **Migration Friendly**: Essential for global → local state transitions
5. **Async Compatibility**: Works seamlessly in async contexts

## Implementation Pattern

### Macro Definition (if not available)
```rust
macro_rules! clone {
    ($($var:ident),+ => $body:expr) => {{
        $(let $var = $var.clone();)+
        $body
    }};
}
```

### Integration with Actor+Relay
```rust
// ✅ Works perfectly with Actor+Relay architecture
impl NovyWaveApp {
    fn files_panel(&self) -> impl Element {
        Column::new()
            .item(self.files_header())
            .item_signal(clone!(self.tracked_files, self.selected_variables => move |_| {
                map_ref! {
                    let files = tracked_files.files.signal_vec_cloned().to_signal_cloned(),
                    let selected = selected_variables.variables.signal_vec_cloned().to_signal_cloned() => {
                        render_file_list(files, selected).into_element()
                    }
                }
            }))
    }
}
```

## Tradeoffs

### Pros
- ✅ Eliminates lifetime complexity
- ✅ Clean, readable code
- ✅ Essential for architectural migrations
- ✅ Works with Actor+Relay patterns
- ✅ Handles complex dependency scenarios

### Cons
- ⚠️ Memory overhead from cloning (usually minimal for domain objects)
- ⚠️ Can mask performance issues if overused with heavy data
- ⚠️ Requires careful consideration of what should be cloned

## Migration Strategy

**During Global State Elimination:**
1. **Identify closure contexts** that access global state
2. **Thread dependencies** to function parameters
3. **Apply clone! macro** to eliminate lifetime issues
4. **Test incrementally** - macro makes refactoring safer
5. **Measure performance** if cloning heavy objects

## Key Insight from NovyWave Experience

**The clone! macro was essential for successfully eliminating global APP_CONFIG state.** Without it, the complexity of manual cloning and lifetime management would have made the migration significantly more difficult and error-prone.

**Pattern Rule:** When eliminating global state, use clone! macro liberally for domain objects - they're designed for cheap cloning and the code clarity benefits far outweigh minimal performance costs.