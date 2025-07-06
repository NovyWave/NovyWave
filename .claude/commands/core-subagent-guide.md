---
description: "Strategic subagent usage patterns for context conservation and parallel work"
allowed-tools: ["Task"]
---

# Strategic Subagent Usage Guide

Extend session effectiveness 2-3x by delegating context-heavy work to subagents.

## When to Use Subagents

**Always delegate these to Task tool subagents:**

### 🔍 Research & Analysis
```bash
# Instead of reading 5+ files yourself:
"Research how TreeView selection works in NovyUI codebase"
"Analyze scroll optimization patterns in competitor tools"
"Find all IconName usage patterns across the project"
```

### 🛠️ Implementation Tasks
```bash
# Instead of doing complex edits yourself:
"Implement CSS Grid optimization for Variables panel layout"
"Add performance measurements to rendering functions"
"Fix compilation errors in button component"
```

### 🔧 Debugging & Investigation
```bash
# Instead of debugging step-by-step yourself:
"Debug why TreeView selection isn't persisting across refreshes"
"Find source of Variables panel overflow issues"
"Investigate Claude Code CLI RangeError causes"
```

### 📊 Testing & Validation
```bash
# Instead of manual testing yourself:
"Test all slash commands and validate YAML frontmatter"
"Verify responsive layout works on different screen sizes"
"Test waveform file loading with various formats"
```

## Subagent Task Patterns

### Research Pattern
```
Description: "Research XYZ implementation"
Prompt: "I need to understand how [feature] works in this codebase. 

Please:
1. Find all relevant files for [feature]
2. Analyze the key patterns and APIs
3. Identify any dependencies or requirements
4. Summarize the architecture in 2-3 key points

Focus on [specific aspect] for my [goal].
Return concise summary, not full file contents."
```

### Implementation Pattern
```
Description: "Implement XYZ feature"
Prompt: "I need you to implement [feature] based on these requirements:

Requirements:
- [requirement 1]
- [requirement 2] 
- [requirement 3]

Please:
1. Make the necessary code changes
2. Test compilation
3. Report any issues found
4. Summarize what was implemented

Working directory: [path]"
```

### Debug Pattern
```
Description: "Debug XYZ issue"
Prompt: "I have this issue: [problem description]

Symptoms:
- [symptom 1]
- [symptom 2]

Please:
1. Investigate the root cause
2. Identify all contributing factors
3. Implement the fix
4. Test the solution
5. Report what was fixed and how

Focus on [specific area] if needed."
```

## Main Session Workflow

1. **Plan & Delegate**: Break work into subagent tasks
2. **Parallel Execution**: Launch multiple subagents for different tasks  
3. **Synthesis**: Combine subagent results
4. **Coordination**: Make architectural decisions
5. **User Communication**: Report progress and results

## Context Conservation Benefits

- **Main session context**: Stays focused on coordination, not implementation details
- **Subagent context**: Each gets fresh context space for their specific task
- **Parallel work**: Multiple investigations/implementations simultaneously
- **Session longevity**: 2-3x longer effective sessions
- **Quality**: Subagents can deep-dive without rushing due to context limits

## Best Practices

✅ **Do:**
- Give subagents clear, specific goals
- Ask for concise summaries, not raw data
- Use multiple subagents for parallel work
- Let subagents handle complex file operations

❌ **Don't:**
- Read large files in main session when subagent could summarize
- Do step-by-step debugging when subagent could investigate
- Manually search through many files
- Implement complex features without delegation

## Example Workflow

```bash
# Traditional approach (burns context fast):
Read file1.rs, file2.rs, file3.rs, analyze patterns, implement changes, test

# Strategic subagent approach:
Task: "Research pattern in file1-3, summarize key insights"
Task: "Implement changes based on these requirements" 
Task: "Test implementation and report results"
# Main session: coordinate, make decisions, communicate with user
```

Use this pattern consistently to maximize session effectiveness and work quality.