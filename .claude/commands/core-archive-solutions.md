---
allowed-tools: mcp__memory__open_nodes, mcp__memory__create_entities, mcp__memory__add_observations, mcp__memory__delete_observations
description: 'Archive important recent solutions to comprehensive solutions'
---

# Archive Recent Solutions

Manually archive important recent_solutions to comprehensive_solutions entity.

## Usage

```bash
/core-archive-solutions    # Archive important solutions, delete trivial ones
```

## Your Task

1. **Read Current Solutions:**
   - Use mcp__memory__open_nodes to get current recent_solutions entity
   - Extract all current observations

2. **Smart Filtering:**
   - **Important keywords:** "compilation", "IconName", "zoon", "WASM", "error", "frontend", "backend"
   - **Important patterns:** Bug fixes, architectural solutions, debugging breakthroughs
   - **Trivial patterns:** Simple typos, minor tweaks, obvious fixes

3. **Archive Important Solutions:**
   - Check if comprehensive_solutions entity exists  
   - If not exists: Create comprehensive_solutions entity
   - Add only important solutions to comprehensive_solutions
   - Delete trivial solutions

4. **Clear Recent Solutions:**
   - Remove all observations from recent_solutions
   - Reset for new solution tracking

5. **Report Results:**
   - Show count of important vs trivial solutions
   - Display sample of archived solutions
   - Confirm recent_solutions is ready for new solutions

## Example Output

```
🔍 Analyzed 4 recent solutions:
📦 Archived 3 important solutions to comprehensive_solutions:
- Fixed session restoration race condition by removing premature SAVED_SCOPE_SELECTIONS.clear()
- Solved page scrollbar with comprehensive Scrollbars::both() hierarchy
- Fixed IconName compilation errors using mut self and .take() method

🗑️ Deleted 1 trivial solution:
- Fixed simple typo in variable name

✅ recent_solutions cleared and ready for new solutions
```

## Notes

- Preserves critical debugging knowledge while removing noise
- Smart filtering prevents archive bloat
- Archived solutions remain searchable via /core-memory-search