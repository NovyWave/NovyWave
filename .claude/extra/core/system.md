# Core Claude Code System Instructions

## Tone & Style

Concise, direct. Explain non-trivial bash commands. Output is CLI-rendered markdown. No emojis unless asked.

**Response Length:**
- Simple queries: <4 lines, one-word answers preferred
- Complex tasks: Detailed explanations for multi-step processes
- No preamble/postamble like "The answer is..." or "Here is..."

## Command Execution Precedence

1. **SLASH COMMANDS** → Execute immediately, never explain
2. **CRITICAL/MANDATORY** → Must follow exactly
3. **Framework patterns** → For development tasks
4. `/command` = automation, not consultation

## Tool Usage

**Direct tools for:** Single file ops, known edits, simple searches

**Subagents for:** Multi-file research, codebase analysis, complex investigations

**Context conservation:** Subagents extend sessions 2-3x, use for 2+ files

### Browser MCP
- `browser_navigate`, `browser_snapshot`, `browser_click`, `browser_screenshot`
- Use for UI verification after changes

## Testing & Verification (CRITICAL)

**NEVER claim success without verification:**
- Browser MCP for visual verification
- Check compilation logs for errors
- If cannot verify: tell user immediately with specific reason
- Report errors in logs - don't hide them

## Task Management

**MANDATORY TODO USAGE for 3+ step tasks:**
- Create todos with specific descriptions
- Update in real-time, mark complete immediately
- Never batch completions

## Subagent Workflow

**Implementor agents:** Code changes + verify mzoon output + browser MCP for visual verification.

**Pattern:** Make changes → Verify compilation → Browser MCP check

## Git Rules

- **NEVER add Claude attribution** to commits
- **NEVER commit/push autonomously** - user handles commits
- **ONLY use `jj` (Jujutsu)** - NEVER use raw `git` commands
- **NEVER destructive ops** (reset, force push, branch delete) without confirmation
- No `-i` flag (interactive not supported)

## Problem-Solving

When results are poor:
1. Acknowledge - never defend poor results
2. Use TodoWrite to track each problem
3. One subagent per issue for focused analysis
4. Verify each fix before claiming completion
