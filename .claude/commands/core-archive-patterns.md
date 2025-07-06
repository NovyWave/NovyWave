---
allowed-tools: mcp__memory__open_nodes, mcp__memory__create_entities, mcp__memory__add_observations, mcp__memory__delete_observations
description: 'Archive daily patterns to comprehensive development patterns'
---

# Archive Daily Patterns

Manually archive current daily_patterns to comprehensive_development_patterns entity.

## Usage

```bash
/core-archive-patterns    # Archive all current daily patterns
```

## Your Task

1. **Read Current Patterns:**
   - Use mcp__memory__open_nodes to get current daily_patterns entity
   - Extract all current observations

2. **Archive to Comprehensive:**
   - Check if comprehensive_development_patterns entity exists
   - If not exists: Create comprehensive_development_patterns entity
   - Add all daily_patterns observations to comprehensive_development_patterns
   - Clear daily_patterns observations (reset to empty for new patterns)

3. **Confirm Archival:**
   - Show user how many patterns were archived
   - Display sample of archived patterns
   - Confirm daily_patterns is ready for new patterns

## Example Output

```
📦 Archived 5 patterns to comprehensive_development_patterns:
- Use IconName enum tokens, never strings for icons
- Use zoon::println!() for WASM logging, never std::println!()
- Use Height::screen() + Height::fill() pattern for layouts
- Always use Width::fill() for responsive design
- Store patterns immediately in Memory MCP after solving bugs

✅ daily_patterns cleared and ready for new patterns
```

## Notes

- This preserves valuable patterns while making room for new ones
- Archived patterns remain searchable via /core-memory-search
- Auto-archival normally triggers at 5 observations, this is manual override