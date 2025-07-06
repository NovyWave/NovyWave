---
allowed-tools: mcp__memory__open_nodes, mcp__memory__create_entities, mcp__memory__add_observations, mcp__memory__delete_observations
description: 'Archive session planning to archived planning'
---

# Archive Session Planning

Manually archive current session_planning to archived_planning entity.

## Usage

```bash
/core-archive-planning    # Archive all current session planning
```

## Your Task

1. **Read Current Planning:**
   - Use mcp__memory__open_nodes to get current session_planning entity
   - Extract all current observations

2. **Archive to Historical:**
   - Check if archived_planning entity exists
   - If not exists: Create archived_planning entity  
   - Add all session_planning observations to archived_planning
   - Clear session_planning observations (reset for new planning)

3. **Confirm Archival:**
   - Show user how many planning items were archived
   - Display sample of archived planning
   - Confirm session_planning is ready for new planning

## Example Output

```
📦 Archived 3 planning items to archived_planning:
- Scrollbar hierarchy analysis: root → layout → panels for overflow containment
- Session restoration strategy: deferred scope selection after file loading
- Panel restructuring approach: fixed headers with scrollable content areas

✅ session_planning cleared and ready for new planning
```

## Notes

- Preserves strategic planning and design decisions
- Maintains historical context for future reference
- Archived planning remains searchable via /core-memory-search
- Auto-archival normally triggers at 5 observations, this is manual override