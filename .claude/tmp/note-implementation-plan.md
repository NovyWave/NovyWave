# /note Command Implementation Plan

## Current Behavior
- Claude Code tries to create new entities instead of using existing ones
- No feedback about which entity received the note

## Improved Behavior

### 1. Keyword Matching Rules
```
"fixed", "solved", "resolved", "corrected" → recent_solutions
"working on", "implementing", "developing" → current_session_state  
"blocked", "stuck", "waiting", "can't" → active_blockers
"always", "never", "must", "pattern", "rule" → daily_patterns
"next", "todo", "will", "then", "after" → next_steps
```

### 2. Entity Detection Logic
```python
def determine_entity(note_text):
    text_lower = note_text.lower()
    
    # Check keywords
    if any(word in text_lower for word in ["fixed", "solved", "resolved"]):
        return ("recent_solutions", True)  # (entity_name, is_focused)
    elif any(word in text_lower for word in ["working on", "implementing"]):
        return ("current_session_state", True)
    # ... etc
    
    # Fallback: create new entity with timestamp
    return (f"note_{datetime.now()}", False)
```

### 3. Feedback Format
```
✓ Note added to 'recent_solutions' (focused)
  This will appear in your /focus output

✓ Note added to 'architecture_notes_2025_01_03' (not focused)
  Use /memory-search to find this note later
```

### 4. Implementation Steps
1. Check if entity exists in Memory MCP
2. Use add_observations for existing entities
3. Use create_entities for new entities
4. Provide clear feedback with entity name and focus status