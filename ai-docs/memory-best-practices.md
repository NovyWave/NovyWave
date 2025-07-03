# Memory Management Best Practices

## Mandatory Session Start Pattern

**EVERY session must begin with:**
```
Remembering previous context...
[run mcp__memory__search_nodes with relevant query]
```

Example queries:
- "NovyWave" - general project context
- "button component" - specific component patterns
- "compilation error" - debugging patterns
- "responsive layout" - layout patterns

## Storage Decision Matrix

### Memory MCP (Persistent Knowledge Graph)
**Store when:**
- Solving bugs or compilation errors (problem + solution)
- Creating new UI patterns or component examples
- Making architectural decisions
- Discovering framework-specific patterns
- Fixing responsive design issues
- Implementing new features

**Storage format:**
- **Entities:** Major concepts (3-5 key observations max)
- **Relations:** Active voice connections
- **Observations:** Atomic facts, not verbose explanations

### CLAUDE.md (Project Rules)
**Store when:**
- General project configuration
- Core development commands
- Critical development rules
- Permanent architectural guidelines

**Keep under 60 lines total**

### docs/ Directory (Detailed Reference)
**Store when:**
- Detailed patterns and examples
- Step-by-step workflows
- Component API documentation
- Framework-specific guides

## Memory Entity Guidelines

### Entity Design
- **Name:** Clear, specific (not generic like "UI patterns")
- **Type:** Descriptive (architecture, component_library, process, etc.)
- **Observations:** 3-5 key facts maximum

### Good Entity Examples
```
Entity: "NovyUI Design System"
Type: component_library
Observations:
- ALL icons use IconName enum tokens for compile-time safety
- Button API: button().label().variant().left_icon(IconName::X).build()
- Responsive: Width::fill() required, Font::new().no_wrap() prevents wrapping
```

### Bad Entity Examples
```
Entity: "UI Stuff" (too generic)
Type: code
Observations: [20+ verbose debugging steps] (too detailed)
```

## Cleanup Patterns

### Regular Maintenance
- Delete entities that become irrelevant
- Consolidate similar entities
- Remove outdated observations
- Limit observations to 5 per entity

### Session Hygiene
- Complete todos immediately after finishing
- Store discoveries while fresh in context
- Don't batch storage - do it immediately
- Prefer atomic facts over verbose explanations

## Query Patterns

### Effective Searches
- Use specific terms: "button component" not "UI"
- Search for problems: "compilation error IconName"
- Look for patterns: "responsive layout Height::fill"

### Context Retrieval
- Start broad: "NovyWave" for general context
- Get specific: "Zoon framework" for technical details
- Problem-focused: "error" for debugging context

## Custom Slash Commands

**Available Commands:**
- `/project:memory-cleanup` - Optimize CLAUDE.md and Memory MCP
- `/project:session-start [term]` - Mandatory context retrieval for session start
- `/project:store-pattern "description"` - Immediately store discoveries in Memory MCP

**Usage Examples:**
```
/project:session-start
/project:session-start "button"
/project:store-pattern "Fixed compilation by adding mut self"
/project:memory-cleanup
```