# CLAUDE.md

Core guidance for Claude Code when working with NovyWave.

<!-- Core System Layer -->
@.claude/extra/core/system.md
@.claude/extra/core/development.md

<!-- Project Configuration -->
@.claude/extra/project/patterns.md

<!-- Technical Reference -->
@.claude/extra/technical/reference.md
@.claude/extra/technical/performance-debugging.md
@.claude/extra/technical/reactive-antipatterns.md
@.claude/extra/technical/lessons.md

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

