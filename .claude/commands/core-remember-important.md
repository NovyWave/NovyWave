# Session Discovery Storage - Use before stopping Claude CLI or calling /clear

Store all important discoveries, patterns, and solutions from current conversation to session notes before ending session.

## Usage

```bash
/core-remember-important        # Store everything important
```

## What Gets Stored

**Automatically detects and stores:**
- Bug fixes and solutions discovered this session
- New patterns or rules learned  
- Important decisions made
- Blockers encountered and resolved
- Next steps for continuation
- Critical file changes or architectural decisions

**Storage Location:**
- **`.claude/session-notes.md`**: Session discoveries with timestamps

## When to Use

**Perfect for:**
- Before ending Claude Code session
- Before calling `/clear` 
- After major breakthroughs or solutions
- When switching to different work

## Implementation

This command should:
1. Analyze the current session for important discoveries
2. Format findings with clear timestamps and context
3. Append to `.claude/session-notes.md` 
4. Include relevant file references and code snippets
5. Focus on actionable insights for future development

## Example Output Format
```markdown
## Session: 2025-01-15 14:30:00

### Bug Fix: WASM Compilation Error
- **Problem**: TreeView component not rendering after Fast2D integration
- **Solution**: Added zoon::println!() debug logging revealed missing Width::fill()
- **File**: frontend/src/components/tree_view.rs:45

### Performance Optimization
- **Discovery**: Virtual list buffering sweet spot is 5-15 elements
- **Impact**: Reduced render time from 200ms to 50ms for 1000+ items
```