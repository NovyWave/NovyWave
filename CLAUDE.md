# CLAUDE.md

Core guidance for Claude Code when working with NovyWave.

<!-- Core System Layer -->
@.claude/extra/core/system.md
@.claude/extra/core/development.md

<!-- Project Configuration -->
@.claude/extra/project/patterns.md
@.claude/extra/project/domain_map.md
@.claude/extra/project/task_execution.md

<!-- Project Specifications -->
@.claude/extra/project/specs/specs.md

<!-- Technical Reference -->
@.claude/extra/technical/reference.md
@.claude/extra/technical/performance-debugging.md
@.claude/extra/technical/reactive-antipatterns.md
@.claude/extra/technical/lessons.md

<!-- Architecture Patterns -->
@.claude/extra/architecture/actor-relay-patterns.md
@.claude/extra/architecture/connection-message-actor-pattern.md

## Actor+Relay Architecture (MANDATORY)

**CRITICAL: NovyWave uses Actor+Relay architecture - NO raw Mutables allowed**

> **📖 Complete API Reference:** See `frontend/src/dataflow/` for full API specification with all methods and the critical "Cache Current Values" pattern. See `docs/actors_relays/actor_relay_architecture.md` for conceptual guidance and architectural patterns.

### Core Architectural Rules

1. **NO RAW MUTABLES:** All state must use Actor+Relay or Atom
   ```rust
   // ❌ PROHIBITED: Raw global mutables
   static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
   static DIALOG_OPEN: Lazy<Mutable<bool>> = lazy::default();
   
   // ✅ REQUIRED: Domain-driven Actors
   struct TrackedFiles {
       files: ActorVec<TrackedFile>,
       file_dropped_relay: Relay<Vec<PathBuf>>,
   }
   
   // ✅ REQUIRED: Atom for local UI
   let dialog_open = Atom::new(false);
   ```

2. **Event-Source Relay Naming (MANDATORY):**
   ```rust
   // ✅ CORRECT: Describe what happened, not what to do
   button_clicked_relay: Relay,              // User clicked button
   file_loaded_relay: Relay<PathBuf>,        // File finished loading
   input_changed_relay: Relay<String>,       // Input text changed
   error_occurred_relay: Relay<String>,      // System error happened
   
   // ❌ PROHIBITED: Command-like naming
   add_file: Relay<PathBuf>,                 // Sounds like command
   remove_item: Relay<String>,               // Imperative style
   set_theme: Relay<Theme>,                  // Action-oriented
   ```

3. **Domain-Driven Design (MANDATORY):**
   ```rust
   // ✅ REQUIRED: Model what it IS, not what it manages
   struct TrackedFiles { ... }              // Collection of files
   struct WaveformTimeline { ... }          // The timeline itself
   struct SelectedVariables { ... }         // Selected variables
   
   // ❌ PROHIBITED: Enterprise abstractions
   struct FileManager { ... }               // Artificial "manager"
   struct TimelineService { ... }           // Unnecessary "service"
   struct DataController { ... }            // Vague "controller"
   ```

4. **Cache Current Values Pattern (CRITICAL):**
   ```rust
   // ✅ ONLY inside Actor loops for event response
   let actor = ActorVec::new(vec![], async move |state| {
       let mut cached_username = String::new();  // Cache values
       let mut cached_message = String::new();
       
       loop {
           select! {
               Some(username) = username_stream.next() => cached_username = username,
               Some(message) = message_stream.next() => cached_message = message,
               Some(()) = send_button_stream.next() => {
                   // Use cached values when responding to events
                   send_message(&cached_username, &cached_message);
               }
           }
       }
   });
   
   // ❌ NEVER cache values anywhere else - use signals instead
   ```

5. **ABSOLUTELY NO FALLBACKS (CRITICAL):**
   **NEVER return fallback values, defaults, or emergency ranges:**
   
   ```rust
   // ❌ ABSOLUTELY PROHIBITED: Any kind of fallback values
   if no_data_available {
       return (0.0, 1.0);  // NO! Even "minimal" fallbacks are forbidden
       return (0.0, 10.0); // NO! Emergency ranges are forbidden
       return SomeDefault::reasonable(); // NO! No fallbacks ever
   }
   
   // ✅ CORRECT: Show explicit loading state or return None
   if no_data_available {
       return None; // Let caller handle appropriately
       // OR show placeholder UI: "Loading..." / "No data available"
       // OR return empty result and let UI show proper state
   }
   ```
   
   **Why NO fallbacks:**
   - User directive: "NO FALBACKKS!! just show placeholder text or error or whatever but NOOOO FALLBACKs ever"
   - Fallbacks mask real data loading issues and create timing bugs
   - Better to show explicit loading states than wrong data
   - Fallbacks interfere with proper reactive data flow

### Migration Status: 74+ Mutables → Actor+Relay
See `docs/actors_relays/novywave/migration_strategy.md` for complete migration plan.

**Phase 1 Targets:**
- TrackedFiles domain (13 mutables → 1 Actor+Relay struct)
- SelectedVariables domain (8 mutables → 1 Actor+Relay struct) 
- WaveformTimeline domain (25 mutables → 1 Actor+Relay struct)

## ReactiveTreeView & Signal Performance Lessons

**CRITICAL: Review `.claude/extra/technical/reactive-antipatterns.md` for comprehensive signal stability patterns**

### Key Antipatterns Discovered

1. **SignalVec → Signal Conversion Instability (NEVER USE):**
   ```rust
   // ❌ CAUSES 20+ renders from single change
   TRACKED_FILES.signal_vec_cloned().to_signal_cloned().map(|files| {...})
   
   // ✅ USE items_signal_vec instead
   .items_signal_vec(TRACKED_FILES.signal_vec_cloned().map(|item| render(item)))
   ```

2. **Downstream Deduplication Fallacy:** Cannot fix signal instability by filtering downstream - fix at source

3. **Zoon Framework Gotchas:**
   - `Text::new().s()` doesn't work - wrap in `El::new().s().child(Text::new())`  
   - Event handlers: `.on_click(|| {})` not `.on_click(|event| {})`
   - Height inheritance: Every container needs `.s(Height::fill())`

### Performance Debugging Success Pattern
1. Add strategic logging (not spam)
2. Count actual data changes vs UI renders  
3. Use browser MCP for real performance validation
4. Fix root causes, not symptoms
5. Test incrementally with side-by-side comparison

**ReactiveTreeView Achievement:** Working prototype with proper signal architecture, 100% performance improvement over broken signal conversion patterns.

## Command Execution Protocol

**CRITICAL BEHAVIORAL RULE**: Slash commands = automation execution, NEVER consultation

**Examples of CORRECT behavior:**
- User types `/core-commit` → Immediately run git analysis commands and present results
- User types `/core-checkpoint` → Immediately execute checkpoint workflow

**Examples of WRONG behavior (never do this):**
- ❌ "Here's how /core-commit works..."
- ❌ "The /core-commit protocol requires..."
- ❌ "You should use /core-commit by..."

**Anti-Consultation Guards**: Command files have explicit enforcement sections to prevent consultation mode

