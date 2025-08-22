# CLAUDE.md

Core guidance for Claude Code when working with NovyWave.

<!-- Core System Layer -->
@.claude/extra/core/system.md
@.claude/extra/core/development.md

<!-- Project Configuration -->
@.claude/extra/project/patterns.md

<!-- Technical Reference -->
@.claude/extra/technical/reference.md

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

