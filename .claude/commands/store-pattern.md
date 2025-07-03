# Store Pattern Command

Immediately store discoveries, solutions, and patterns in Memory MCP.

## What it does:

**Quick storage of important discoveries:**

1. **Immediate Storage Triggers:**
   - Bug solutions and compilation fixes
   - New UI patterns or component usage
   - Architectural decisions and rationale
   - Framework-specific patterns discovered
   - Responsive design solutions
   - Performance optimizations

2. **Storage Format:**
   - Create focused entities (3-5 observations max)
   - Use atomic, actionable observations
   - Connect to existing entities via relations
   - Avoid verbose debugging details

3. **Auto-categorization:**
   - **Architecture:** High-level decisions and patterns
   - **Component Library:** UI patterns and API usage
   - **Process:** Development workflow and testing
   - **Bug Solution:** Specific problem-solution pairs

## Usage:

```
/project:store-pattern "description of what was discovered/solved"
```

Examples:
- `/project:store-pattern "Fixed IconName enum compilation by adding mut self to build()"`
- `/project:store-pattern "Responsive layout requires Width::fill() + Height::screen() pattern"`
- `/project:store-pattern "NovyUI buttons use .left_icon(IconName::X) not string literals"`

**Use immediately after solving problems or discovering new patterns.**