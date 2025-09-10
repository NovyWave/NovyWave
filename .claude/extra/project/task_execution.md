# Task Execution Guide - NovyWave

**PURPOSE:** Immediate reference for efficient task execution using domain map and architectural patterns.

## Instant Task Analysis Protocol

### Step 1: Domain Identification (5 seconds)
```bash
# Check domain_map.md for affected domains:
# 1. File Management (tracked_files.rs + 3 supporting)
# 2. Variable Selection (selected_variables.rs + 2 supporting) 
# 3. Timeline Visualization (visualizer/timeline/ - 8 files)
# 4. Canvas Rendering (visualizer/canvas/ - 5 files)
# 5. Platform Abstraction (platform/ - 3 files)
```

### Step 2: Architecture Pattern Selection (10 seconds)
```bash
# Required patterns from domain_map.md:
# - Actor+Relay for domain state
# - Event-source relay naming (file_dropped_relay NOT add_file_relay)
# - Cache Current Values ONLY inside Actor loops
# - Public field architecture (direct access, no getters)
```

### Step 3: Implementation Strategy (choose one)

#### A. Single Domain Change (Direct Tools)
```bash
# Use when: Simple changes within one domain
# Tools: Read, Edit, Grep specific files
# Example: Add new relay field to TrackedFiles
```

#### B. Multi-Domain or Complex Change (Subagent)
```bash
# Use when: Cross-domain coordination, performance issues, architectural compliance
# Pattern: Task tool with domain-specific analysis
# Example: "Analyze variable filtering performance in SelectedVariables domain"
```

#### C. UI/UX Change (TodoWrite + Mixed)
```bash
# Use when: Multiple UI files, layout changes, responsive design
# Pattern: TodoWrite for tracking + combination of direct tools and subagents
# Example: Panel layout changes affecting multiple domains
```

## Quick Reference Patterns

### Actor+Relay Creation Template
```rust
// Event-source relays (describe what happened)
let (file_dropped_relay, mut file_dropped_stream) = relay();

// Actor with Cache Current Values pattern
let actor = Actor::new(initial_state, async move |state| {
    // Cache values as they flow through streams
    let mut cached_value = initial_value;
    
    loop {
        select! {
            Some(new_value) = value_stream.next() => cached_value = new_value,
            Some(event) = event_stream.next() => {
                // Use cached values for event response
                process_event(cached_value, event);
                state.set_neq(updated_state);
            }
        }
    }
});
```

### Signal Chain Best Practices
```rust
// ✅ CORRECT: Direct signal access
domain.actor.signal().map(|state| render(state))

// ✅ CORRECT: items_signal_vec for collections  
.items_signal_vec(domain.items.signal_vec().map(|item| render_item(item)))

// ❌ WRONG: SignalVec conversion antipattern
domain.items.signal_vec().to_signal_cloned() // Causes 20+ renders per change
```

### Cross-Domain Integration
```rust
// Context object pattern for utility functions
struct DomainContext {
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    waveform_timeline: WaveformTimeline,
}

// Use self for domain access (eliminates parameter cascading)
impl DomainContext {
    fn utility_function(&self) -> Result<T> {
        let files = self.tracked_files.files_vec_signal.get_cloned();
        // Access other domains via self.domain_name
    }
}
```

## Integration Points Quick Reference

### File Management → Variable Selection
- File loading triggers variable extraction
- File removal triggers variable cleanup  
- Smart labeling considers file contexts

### Variable Selection → Timeline
- Format changes sent via `variable_format_updated_relay`
- Selected variables affect timeline data requests

### Timeline → Canvas  
- TimelineContext bridges domain state to rendering
- Coordinate translation via TimelineCoordinates

### All Domains → Configuration
- Domain changes trigger ConfigSaver debounced persistence
- Config restoration populates domain actors on startup

### All Domains → Dragging
- Panel dimensions managed through DraggingSystem
- Cache Current Values pattern for smooth interactions

## Verification Checklist

### Before Implementation
- [ ] Identified affected domain(s) from domain_map.md
- [ ] Selected appropriate Actor+Relay pattern
- [ ] Checked integration points with other domains
- [ ] Planned verification approach (browser MCP)

### During Implementation  
- [ ] Following event-source relay naming
- [ ] Using Cache Current Values only in Actor loops
- [ ] No raw Mutables introduced
- [ ] Public field architecture maintained

### After Implementation
- [ ] Compilation successful (check dev_server.log)
- [ ] Browser MCP testing confirms functionality
- [ ] No architectural violations introduced
- [ ] Performance acceptable (no signal cascades)

## Anti-Patterns to Avoid

### NEVER Create
- [ ] Manager/Service/Controller objects
- [ ] Raw global Mutables 
- [ ] SignalVec→Signal conversions
- [ ] Command-style relay naming
- [ ] Data bundling structs forcing unrelated updates

### NEVER Use  
- [ ] zoon::Task for event handling (use Actor internal relays)
- [ ] Fallback values or emergency defaults
- [ ] Getter methods wrapping public fields
- [ ] Complex signal coordination (use on-demand computation)

## Emergency Debugging

### Performance Issues
1. Check for SignalVec conversion antipatterns in domain_map.md
2. Use browser MCP for real performance measurement  
3. Reference timeline domain performance patterns
4. Check for signal cascade multiplication

### Compilation Errors
1. Verify Actor+Relay patterns from domain_map.md
2. Check event-source relay naming conventions
3. Ensure no raw Mutable introductions
4. Verify lifetime issues with domain parameter passing

### Functionality Issues
1. Trace data flow using domain integration points
2. Check Actor event processing loops
3. Verify signal chain stability
4. Use browser MCP for runtime behavior validation

This guide integrates with CLAUDE.md and domain_map.md to provide instant context and prevent architectural violations during development.