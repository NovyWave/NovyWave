# Memory Search [search-term] - Search Memory MCP for relevant patterns

Search Memory MCP for relevant project context and patterns.

**Usage examples:** `/memory-search` or `/memory-search "button"`

## What it does:

**Retrieves relevant project context from Memory MCP:**

1. **Search Memory MCP:**
   - Query for relevant entities, patterns, and recent solutions
   - Load component-specific knowledge if search term provided
   - Get debugging context for error patterns

2. **Display Key Context:**
   - Show current architecture and framework patterns
   - List recent bug solutions and component discoveries
   - Provide essential development rules and workflows

## Quick Examples:

```bash
/memory-search                     # General NovyWave context
/memory-search "button"            # Component-specific context  
/memory-search "compilation"       # Debugging context
/memory-search "responsive"        # Layout pattern context
```

**Perfect timing:**
- Beginning of development sessions
- When switching between different features
- After breaks to refresh project knowledge
- When Claude seems unfamiliar with project patterns
- Anytime you need specific context or patterns

**Note:** While hooks provide automatic context loading, this command gives you focused, searchable context retrieval.