# General Development Practices

## Following Conventions

When making changes to files, first understand the file's code conventions. Mimic code style, use existing libraries and utilities, and follow existing patterns.

- NEVER assume that a given library is available, even if it is well known. Whenever you write code that uses a library or framework, first check that this codebase already uses the given library. For example, you might look at neighboring files, or check the package.json (or cargo.toml, and so on depending on the language).
- When you create a new component, first look at existing components to see how they're written; then consider framework choice, naming conventions, typing, and other conventions.
- When you edit a piece of code, first look at the code's surrounding context (especially its imports) to understand the code's choice of frameworks and libraries. Then consider how to make the given change in a way that is most idiomatic.
- Always follow security best practices. Never introduce code that exposes or logs secrets and keys. Never commit secrets or keys to the repository.

## Code Style

- IMPORTANT: DO NOT ADD ***ANY*** COMMENTS unless asked

## Refactoring Rules

**ATOMIC CODE MOVEMENT - NEVER BREAK COMPILATION:**
1. Copy complete code blocks to destination files first
2. Verify compilation succeeds after each copy
3. Only then remove code from source files
4. NEVER create placeholder functions or empty stubs
5. NEVER rename types with aliases (e.g., `Signal as DataSignal`) - move code directly
6. Always preserve exact functionality during moves

## Task Management

You have access to the TodoWrite and TodoRead tools to help you manage and plan tasks. Use these tools VERY frequently to ensure that you are tracking your tasks and giving the user visibility into your progress.

These tools are also EXTREMELY helpful for planning tasks, and for breaking down larger complex tasks into smaller steps. If you do not use this tool when planning, you may forget to do important tasks - and that is unacceptable.

It is critical that you mark todos as completed as soon as you are done with a task. Do not batch up multiple tasks before marking them as completed.

## Proactiveness

You are allowed to be proactive, but only when the user asks you to do something. You should strive to strike a balance between:
1. Doing the right thing when asked, including taking actions and follow-up actions
2. Not surprising the user with actions you take without asking

For example, if the user asks you how to approach something, you should do your best to answer their question first, and not immediately jump into taking actions.

3. Do not add additional code explanation summary unless requested by the user. After working on a file, just stop, rather than providing an explanation of what you did.

## Git Operations

### Committing Changes

When the user asks you to create a new git commit, follow these steps carefully:

1. Run the following bash commands in parallel:
   - Run a git status command to see all untracked files
   - Run a git diff command to see both staged and unstaged changes
   - Run a git log command to see recent commit messages

2. Analyze all staged changes and draft a commit message:
   - Summarize the nature of the changes
   - Check for any sensitive information
   - Draft a concise (1-2 sentences) commit message
   - Ensure it accurately reflects the changes

3. Run the following commands in parallel:
   - Add relevant untracked files to the staging area
   - Create clean commit with no Claude mentions or boilerplate
   - Run git status to verify success

Important notes:
- NEVER update the git config
- NEVER run additional commands to read or explore code
- DO NOT push to the remote repository unless explicitly asked
- Never use git commands with the -i flag
- **CRITICAL: NEVER perform destructive git operations (reset, rebase, force push, branch deletion, stash drop) without explicit user confirmation**
- **User lost hours of work from uncommitted changes - always confirm before any operation that could lose data**
- **Only exceptions: /core-checkpoint and /core-commit commands where destruction is part of expected flow, but still be careful**

### Pull Requests

Use the gh command via the Bash tool for ALL GitHub-related tasks.

When creating a pull request:
1. Run commands in parallel to understand current state
2. Analyze all changes that will be included
3. Create PR using gh pr create with proper formatting

## Hooks

Users may configure 'hooks', shell commands that execute in response to events like tool calls, in settings. If you get blocked by a hook, determine if you can adjust your actions in response to the blocked message. If not, ask the user to check their hooks configuration.

## Do's and Don'ts

### Important Reminders
- Do what has been asked; nothing more, nothing less
- NEVER create files unless they're absolutely necessary
- ALWAYS prefer editing an existing file to creating a new one
- NEVER proactively create documentation files (*.md) or README files unless explicitly requested

### Claude Code System Maintenance

**IMPORTANT: When updating Claude Code infrastructure files, always update the human documentation:**

- Commands added/modified → Update `docs/working-with-claude.md`
- Memory system changes → Update `docs/working-with-claude.md` 
- Hook system changes → Update `docs/working-with-claude.md`
- New slash commands → Update `docs/working-with-claude.md`

The human documentation must stay synchronized with the actual system capabilities so users know what's available.

### Planning
- Use the Task tool when you are in plan mode
- Only use exit_plan_mode tool when planning implementation steps for code writing tasks
- For research tasks (gathering information, searching, reading), do NOT use exit_plan_mode