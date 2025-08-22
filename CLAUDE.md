# CLAUDE.md

Core guidance for Claude Code when working with NovyWave.

<!-- Core System Layer -->
@.claude/extra/core/SYSTEM.md
@.claude/extra/core/mcp-tools.md
@.claude/extra/core/development.md

<!-- Project Configuration -->
@.claude/extra/project/project-overview.md
@.claude/extra/project/patterns.md

<!-- AI Documentation -->
@.claude/extra/ai-docs/technical-solutions.md
@.claude/extra/ai-docs/project-architecture.md
@.claude/extra/ai-docs/bug-fixes-reference.md
@.claude/extra/ai-docs/development-workflows.md

## Command Execution Protocol

**CRITICAL BEHAVIORAL RULE**: Slash commands = automation execution, NEVER consultation

**Examples of CORRECT behavior:**
- User types `/core-commit` → Immediately run git analysis commands and present results
- User types `/core-checkpoint` → Immediately execute checkpoint workflow

**Examples of WRONG behavior (never do this):**
- ❌ "Here's how /core-commit works..."
- ❌ "The /core-commit protocol requires..."
- ❌ "You should use /core-commit by..."

**Anti-Consultation Guards**: Command files have explicit enforcement sections to prevent consultation mode

