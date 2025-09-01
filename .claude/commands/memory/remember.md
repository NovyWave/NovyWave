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

**Note:** The command automatically prepends "remember " to your argument if it doesn't already start with "remember", so you can write naturally:

```bash
/memory:remember that CONFIG_LOADED should be checked before saving
/memory:remember to use Actor+Relay instead of raw mutables  
/memory:remember TreeView backgrounds need min-width: max-content + width: 100%
```

## Your Task

**Implementation Note:** Check if the user's argument starts with "remember". If not, automatically prepend "remember " before processing. This allows natural usage like `/memory:remember to check CONFIG_LOADED` without redundant "remember" words.

### MANDATORY Duplication Prevention:

**CRITICAL: Before adding ANY content, perform semantic duplication detection:**

1. **Search for related concepts (not just exact text):**
   ```bash
   # Search for topic clusters and related concepts
   rg -i "key_topic.*related|related.*key_topic" .claude/extra/ -l
   rg -i "similar.*concept.*words" .claude/extra/ -l  
   rg "structural.*patterns" .claude/extra/ -A 3 -B 1
   
   # Check section headers for conceptual overlap
   rg "^## |^### " .claude/extra/ | grep -i "topic_area"
   rg "MANDATORY|CRITICAL|NEVER" .claude/extra/ -l
   ```

2. **Overlap Assessment Questions:**
   - Does this CONCEPT already exist in different wording elsewhere?
   - Would this create competing authoritative sources?
   - Should existing content be updated instead of adding new?
   - Are we fragmenting well-organized existing content?

3. **File Purpose Validation:**
   - **development.md**: HOW to develop (practices, workflows, code style rules)
   - **technical/**: WHAT patterns to use (specific solutions, debugging, reference)
   - **architecture/**: WHY patterns exist (architectural rules, mandatory patterns, core principles)
   - **project/**: WHEN/WHERE to apply (NovyWave-specific configurations, framework usage)

4. **Destination Justification Required:**
   - Why this specific file instead of others?
   - How does this fit the file's specific purpose?
   - What cross-references need updating?
   - Is this the single authoritative source for this concept?

### Smart Content Placement:

**ONLY after duplication prevention checks above:**

1. **Determine best location based on content type:**
   - **Core practices/workflows** → `.claude/extra/core/development.md` or `.claude/extra/core/system.md`
   - **Architectural patterns/rules** → `.claude/extra/architecture/actor-relay-patterns.md` or new architecture file
   - **Technical solutions/debugging** → `.claude/extra/technical/lessons.md` or existing technical files
   - **Project-specific patterns** → `.claude/extra/project/patterns.md`
   - **Critical patterns** → Add directly to main CLAUDE.md section

2. **Update existing content instead of duplicating:**
   - Prefer enhancing existing sections over creating new ones
   - Use cross-references instead of copying content
   - Consolidate related concepts into single authoritative sources

3. **Format simply:**
   - Use clear headers like `### Pattern Name` or `### Lesson`
   - Write actionable content with code examples when helpful
   - No timestamps or complex structure - just clear, useful information

4. **File Management:**
   - Append to existing files when content fits naturally
   - Create new files only if the topic is substantial and distinct
   - If creating new file, add import to CLAUDE.md: `@.claude/extra/{subfolder}/{filename}.md`

5. **Confirmation:**
   - Show what was remembered
   - Show which file it was added to
   - Explain why this location vs alternatives
   - Confirm it's now part of Claude's context without duplicating elsewhere

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