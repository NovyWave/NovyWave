---
description: 'Add new knowledge or patterns to Claude memory for this project'
---

# Memory: Remember

**Command:** `/memory:remember`

Add new knowledge, patterns, or lessons learned to Claude's memory for this project. This creates persistent context that will be available in future sessions.

## Usage

```bash
/memory:remember <knowledge_to_remember>
```

## Examples

```bash
/memory:remember Use saturating_sub() instead of - for counts to avoid integer overflow panics
/memory:remember TreeView backgrounds need min-width: max-content + width: 100% for proper scrolling
/memory:remember Always check CONFIG_LOADED.get() before triggering config save to prevent startup overwrites
```

## Your Task

### Smart Content Placement:

1. **Analyze the provided knowledge and determine the best location:**
   - **Core practices/workflows** → `.claude/extra/core/development.md` or `.claude/extra/core/system.md`
   - **Project-specific patterns** → `.claude/extra/project/patterns.md` or new project file if substantial
   - **Technical lessons/debugging** → `.claude/extra/technical/lessons.md` or existing technical files
   - **Critical patterns** → Add directly to main CLAUDE.md section

2. **Format simply:**
   - Use clear headers like `### Pattern Name` or `### Lesson`
   - Write actionable content with code examples when helpful
   - No timestamps or complex structure - just clear, useful information

3. **File Management:**
   - Append to existing files when content fits naturally
   - Create new files only if the topic is substantial and distinct
   - If creating new file, add import to CLAUDE.md: `@.claude/extra/{subfolder}/{filename}.md`

4. **Confirmation:**
   - Show what was remembered
   - Show which file it was added to
   - Confirm it's now part of Claude's context

## Memory Organization

Memory is organized in `.claude/extra/` with subdirectories:

- **`core/`** - Core development practices, workflows, system instructions
- **`project/`** - Project-specific configurations, patterns, domain knowledge  
- **`technical/`** - Technical reference, debugging patterns, performance lessons

Each memory entry is simple and actionable:
- Clear descriptive header
- Focused, useful content
- Code examples where helpful
- No bureaucratic overhead