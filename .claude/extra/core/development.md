# Development Practices & Workflows

## Following Conventions

When making changes to files, first understand the file's code conventions. Mimic code style, use existing libraries and utilities, and follow existing patterns.

- NEVER assume that a given library is available, even if it is well known. Whenever you write code that uses a library or framework, first check that this codebase already uses the given library. For example, you might look at neighboring files, or check the package.json (or cargo.toml, and so on depending on the language).
- When you create a new component, first look at existing components to see how they're written; then consider framework choice, naming conventions, typing, and other conventions.
- When you edit a piece of code, first look at the code's surrounding context (especially its imports) to understand the code's choice of frameworks and libraries. Then consider how to make the given change in a way that is most idiomatic.
- Always follow security best practices. Never introduce code that exposes or logs secrets and keys. Never commit secrets or keys to the repository.

## Code Style

- IMPORTANT: DO NOT ADD ***ANY*** COMMENTS unless asked

## Code Style & Patterns

> **📖 Detailed Coding Patterns:** See @.claude/extra/technical/coding-patterns.md for comprehensive coding patterns, architectural rules, and best practices including Actor+Relay patterns, WASM error handling, modern Rust formatting, and antipattern prevention.

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

## Systematic Multi-Session Task Methodology

### When to Use This Methodology

**Apply this systematic approach for:**
- **Complex tasks spanning multiple sessions** - Architecture migrations, major refactors, large-scale cleanups
- **Tasks with high regression risk** - Changes that could introduce new antipatterns
- **Warning/error resolution campaigns** - Systematic elimination of compiler warnings or errors
- **Large codebase updates** - Changes affecting 10+ files or multiple domains
- **Quality improvement initiatives** - Code cleanup, performance optimization, architectural compliance

**Pattern Recognition:** If a task requires more than 2-3 sessions or affects critical architecture, use this methodology.

### 5-Phase Safety Methodology

**Critical Success Pattern: Preparation → Implementation → Verification → Monitoring → Documentation**

#### Phase 1: Foundation Reading (MANDATORY)

**Before touching any code:**
```
[ ] Read relevant antipattern documentation
[ ] Study correct pattern examples (e.g., chat_example.md)
[ ] Review architectural rules (CLAUDE.md + .claude/extra/)
[ ] Check current system state (logs, warnings, errors)
[ ] Identify specific target scope for this session
[ ] Create TodoWrite plan with specific verification checkpoints
```

**Key Principle:** Never start implementation without understanding the correct patterns and current system state.

#### Phase 2: Systematic Implementation

**Session Scope Rules:**
- **Single focus area** - Never work on multiple architectural domains simultaneously
- **Limited scope** - Maximum 10-15 related changes per session
- **TodoWrite tracking** - Every step tracked with intermediate verification
- **File-by-file approach** - Complete and verify each file before moving to next
- **Frequent compilation checks** - Verify after each significant change

#### Phase 3: Anti-Regression Verification (CRITICAL)

**After each session, verify no new antipatterns introduced:**
```
[ ] Check for new antipattern indicators (grep/rg commands)
[ ] Verify architectural compliance (Actor+Relay, no raw mutables, etc.)
[ ] Confirm compilation success
[ ] Document change impact (warning counts, error elimination)
[ ] Test critical functionality if applicable
```

**🚨 REGRESSION INDICATORS - STOP AND FIX:**
- **Error/warning count increased** - New problems introduced
- **New antipattern code patterns** - Architectural violations
- **Compilation failures** - Fix immediately before continuing

#### Phase 4: Documentation Maintenance

**Keep documentation synchronized:**
```
[ ] Update relevant documentation with resolved issues
[ ] Mark completed patterns with ✅ RESOLVED
[ ] Update success metrics and baselines
[ ] Document new antipatterns discovered
[ ] Commit documentation changes with descriptive messages
```

#### Phase 5: Ongoing Monitoring

**Prevent regression and architectural drift:**
```
[ ] Weekly/bi-weekly system health checks
[ ] Monitor trending metrics (warnings, errors, performance)
[ ] Review recent commits for antipattern introduction
[ ] Update methodology based on lessons learned
```

### Session Workflow Template

**Copy this template for complex multi-session tasks:**

```
## Multi-Session Task: [Task Name] - Session [X] - [Date]

### Preparation Phase
[ ] Current system state: [warnings/errors count, compilation status]
[ ] Target scope: [specific area/domain/files]
[ ] Re-read relevant antipatterns documentation
[ ] Review correct patterns for this domain
[ ] Plan specific changes for this session

### Implementation Phase
[ ] Change 1: [specific change] - [file/location]
[ ] Change 2: [specific change] - [file/location]
[ ] Change 3: [specific change] - [file/location]
[ ] Compilation check after each change
[ ] Progress metric check after each change

### Verification Phase  
[ ] NO new antipatterns introduced: [specific checks]
[ ] Compilation success: [status]
[ ] Target metrics improvement: [before → after counts]
[ ] Critical functionality verification: [if needed]

### Documentation Phase
[ ] Update relevant docs with progress
[ ] Mark resolved items
[ ] Update success metrics
[ ] Commit with descriptive message

### Session Results
- **Changes Made:** [summary]
- **Files Modified:** [list]
- **Metrics Impact:** [before → after]
- **Verification Status:** ✅ PASS / ❌ FAIL
- **Next Session Plan:** [what to tackle next]
```

### Emergency Recovery Pattern

**If verification reveals new problems:**
1. **STOP** all implementation work immediately
2. **Revert** to last known-good state: `git reset --hard <commit>`
3. **Re-read** documentation to understand what went wrong
4. **Re-plan** with smaller, safer scope
5. **Use TodoWrite** to track micro-steps
6. **Get verification** at each tiny step before proceeding

### Success Indicators

**✅ METHODOLOGY WORKING:**
- **Steady progress** - Target metrics improving over sessions
- **No regressions** - New problems not introduced during fixes
- **Sustainable pace** - Can work consistently without burnout or confusion
- **Clear direction** - Always know what to do next
- **Quality maintained** - Compilation success and functionality preserved

**❌ METHODOLOGY FAILING:**
- **Chaos symptoms** - Lost track of what was changed or needs changing
- **Regression introduction** - New problems appearing during "fixes"
- **Overwhelming scope** - Trying to change too much at once
- **Verification skipping** - Not checking after each step

**Recovery:** Return to Phase 1 (Foundation Reading) and restart with smaller scope.

### Multi-Session Coordination

**Between sessions:**
- **Commit all changes** with descriptive messages
- **Document session results** using template above
- **Update tracking documents** (like hunt_warnings.md)
- **Plan next session scope** based on current progress

**Session startup checklist:**
- **Review previous session results**
- **Check current system state** (may have changed)
- **Confirm no regressions** since last session
- **Plan current session scope** (small, focused)

This methodology ensures systematic progress on complex tasks while preventing the introduction of new problems during the resolution process.

