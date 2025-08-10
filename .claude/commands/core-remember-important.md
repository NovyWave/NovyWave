# /core-remember-important Command

## Purpose  
Store session discoveries to permanent memory before ending sessions

## CRITICAL: Slash Command = Automation

**NEVER provide consultation when user types `/core-remember-important`**
**ALWAYS execute the storage workflow immediately**

## Triggers - Use This Command When You:
- Solve any bug or compilation error
- Create new UI patterns or component examples  
- Make architectural decisions
- Discover framework-specific patterns (Zoon, NovyUI, Fast2D)
- Fix responsive design issues
- Complete feature implementations with reusable patterns

## Workflow

### 1. Analyze Current Session
Identify important discoveries from current conversation:
- Bug fixes and their solutions
- New patterns or techniques discovered
- Framework-specific insights
- Performance optimizations
- UI/UX solutions

### 2. Storage Target
Append discoveries to: `.claude/session-logs/discoveries.md`

### 3. Format Template
```markdown
## Session Discovery: [Date/Time]

### Problem/Context
Brief description of the issue or goal

### Solution/Pattern  
Technical details of the solution

### Code Example (if applicable)
```rust
// Example implementation
```

### Impact/Lesson
Why this discovery is important for future sessions
```

### 4. Update Storage File
Use Write or Edit tool to append new discoveries to the target file.

## Storage Strategy
- **Session-specific**: Temporary discoveries and debugging solutions
- **Pattern-worthy**: Solutions that apply beyond current task
- **Framework insights**: MoonZoon, Zoon, NovyUI, Fast2D specific learnings
- **Performance lessons**: Optimization techniques and anti-patterns

## Anti-Consultation Guard
This command MUST execute storage immediately. Never explain how memory storage works unless explicitly asked after completion.