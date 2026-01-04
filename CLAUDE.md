# CLAUDE.md

Core guidance for Claude Code when working with NovyWave.

<!-- Core (always loaded) -->
@.claude/extra/core/system.md
@.claude/extra/core/development.md

<!-- Project (always loaded) -->
@.claude/extra/project/patterns.md
@.claude/extra/project/domain_map.md
@.claude/extra/project/task_execution.md

<!-- Technical (always loaded) -->
@.claude/extra/technical/reference.md
@.claude/extra/technical/performance-debugging.md

<!-- ON-DEMAND: Load when working on UI/UX features -->
<!-- @.claude/extra/project/specs/specs.md -->

## State Management

**Use standard Zoon primitives for state management:**

```rust
// Local/shared mutable state
let dialog_visible = Mutable::new(false);
let selected_files = MutableVec::new();

// Async communication between components
let (sender, receiver) = futures::channel::mpsc::unbounded::<Message>();
```

### Core Rules

1. **ABSOLUTELY NO FALLBACKS (CRITICAL):**
   ```rust
   // ❌ PROHIBITED: Any kind of fallback values
   if no_data_available {
       return (0.0, 1.0);  // NO! Even "minimal" fallbacks are forbidden
   }

   // ✅ CORRECT: Show explicit loading state or return None
   if no_data_available {
       return None; // Let caller handle appropriately
   }
   ```

2. **Domain-Driven Design:**
   ```rust
   // ✅ Model what it IS
   struct TrackedFiles { ... }
   struct WaveformTimeline { ... }

   // ❌ PROHIBITED: Enterprise abstractions
   struct FileManager { ... }
   struct TimelineService { ... }
   ```

## ReactiveTreeView & Signal Performance Lessons

**CRITICAL: Review `.claude/extra/core/development.md` for comprehensive signal stability patterns**

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

