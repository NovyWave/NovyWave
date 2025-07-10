# Intelligent Session Memory - Use before stopping Claude CLI or calling /clear

Store all important discoveries, patterns, and solutions from current conversation to persistent memory before ending session.

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

**Storage Locations:**
- **Memory MCP**: Session discoveries, solutions, patterns
- **CLAUDE.md**: Permanent rules if framework/project-wide
- **Focus Context**: Current state for immediate pickup

## When to Use

**Perfect for:**
- Before ending Claude Code session
- Before calling `/clear` 
- After major breakthroughs or solutions
- When switching to different work

## Intelligence

- Analyzes conversation for important learnings
- Avoids storing trivial or duplicate information  
- Updates existing patterns rather than creating duplicates
- Preserves context for seamless session continuation