# Compaction-Safe Development Patterns

## The Problem

Conversation compaction can cause Claude to lose critical context about:
- Interactive workflow requirements
- Safety constraints and user confirmation needs
- Complex behavioral rules and state dependencies
- Temporary debugging state and current focus

## Core Patterns

### 1. Embed Critical Rules in Command Files

**Problem**: After compaction, Claude may forget that /core-commit requires user confirmation.

**Solution**: Store behavioral rules directly in command markdown:
```markdown
### ⚠️ CRITICAL IMPLEMENTATION RULES

**RULE 2 - INTERACTIVE WORKFLOW**: NEVER execute git commands without user confirmation:
- **ALWAYS** present analysis and suggested commit message FIRST
- **ALWAYS** wait for user approval: y/n/custom response
- **NEVER** auto-execute git commands even after conversation compaction
```

**Why it works**: Command files persist across sessions and compaction cycles.

### 2. Memory MCP for Session State

**Use Memory MCP for**:
- Current work state and focus
- Recent solutions and blockers
- Essential daily patterns
- Critical debugging discoveries

**Store immediately when**:
- Solving bugs or compilation errors
- Discovering framework-specific patterns
- Making architectural decisions
- Encountering workflow blockers

### 3. CLAUDE.md for Permanent Rules

**Use CLAUDE.md imports for**:
- Framework-specific patterns
- Project architecture decisions
- Development workflow requirements
- Tool usage policies

**Example**: `@.claude/core/SYSTEM.md` contains universal Claude behavior rules.

### 4. Documentation for Complex Patterns

**Create docs/ files for**:
- Multi-step procedures that need human reference
- Complex debugging techniques learned
- Architectural patterns and solutions
- Project-specific development rules

### 5. Command Validation Patterns

**For critical commands, add explicit validation**:
```markdown
6. **Present Options & Wait for User Response:**
   - Show analysis results
   - Present recommended action with clear options
   - **STOP and wait for user input**
   - Do NOT proceed until user responds with y/n/custom/etc.
```

## Specific Anti-Patterns to Avoid

### ❌ Relying Only on Conversation Memory
```
# This context gets lost during compaction:
User: "Remember to always ask for confirmation before git operations"
Claude: "I'll remember that"
# After compaction: Claude auto-executes git commands
```

### ❌ Temporary State in Memory Only
```
# This debugging state gets lost:
Claude stores in memory: "Currently debugging icon rotation bug"
# After compaction: Claude doesn't know what was being debugged
```

### ❌ Complex Workflows Without Documentation
```
# Multi-step process gets fragmented:
Step 1: Analyze CHECKPOINT
Step 2: Check for splits  
Step 3: Wait for user confirmation
Step 4: Execute
# After compaction: Steps get executed out of order
```

## Recommended Compaction-Safe Architecture

### Immediate Storage (Memory MCP)
- Current session state
- Recent solutions and blockers
- Active debugging focus
- Next immediate steps

### Persistent Rules (Command Files)
- Critical behavioral constraints
- Interactive workflow requirements
- Safety checks and confirmations
- Never-violate principles

### Permanent Patterns (CLAUDE.md + docs/)
- Framework-specific rules
- Project architecture
- Development standards
- Reusable solutions

### Human Reference (docs/)
- Complex multi-step procedures
- Troubleshooting guides
- Pattern explanations
- Historical context

## Implementation Strategy

1. **Identify Critical Behaviors**: What must NEVER be forgotten even after compaction?
2. **Embed in Commands**: Put safety rules directly in command files
3. **Document Patterns**: Create human-readable docs for complex procedures
4. **Update Memory**: Store current state and recent discoveries
5. **Test Compaction**: Verify behavior survives context loss

## Examples of Good Compaction-Safe Design

### /core-commit Command
- **RULE 1** embedded in command file: Always analyze CHECKPOINT contents
- **RULE 2** embedded in command file: Never auto-execute without confirmation
- **Workflow steps** explicitly numbered and documented
- **Safety checks** built into the process

### Memory MCP System
- **Focused entities** with automatic updates
- **Daily patterns** that persist critical rules
- **Recent solutions** that prevent repeating mistakes
- **Session state** that maintains current focus

This pattern ensures Claude maintains consistent behavior even after conversation compaction.