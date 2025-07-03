# Modular Structure Example

## 🏗️ Proposed Directory Structure

```
.claude/
├── core/                          # ✅ Copy to ANY project
│   ├── SYSTEM.md                  # Claude's core behavior
│   ├── memory-management.md       # Focus system & smart archiving
│   ├── mcp-tools.md              # Memory & Browser MCP usage
│   ├── development-workflow.md    # Git, testing, general dev
│   └── commands/
│       ├── focus.md              
│       ├── note.md               
│       ├── memory-search.md      
│       └── memory-cleanup.md     
│
├── frameworks/                    # ✅ Copy to same-framework projects
│   └── moonzoon/
│       ├── FRAMEWORK.md          # MoonZoon overview
│       ├── zoon-patterns.md      # UI patterns
│       ├── wasm-debugging.md     # Compilation tips
│       └── commands/
│           ├── start.template.md # With {{PROJECT_NAME}} placeholders
│           └── stop.template.md  
│
├── PROJECT.md                     # ❌ Project-specific only
├── project-docs/                  # ❌ Project-specific only
│   ├── architecture.md
│   ├── novyui-components.md
│   └── domain-rules.md
│
├── CLAUDE.md                      # 🔧 Simple assembly file
├── ai-memory.json                 # 🔧 Project-specific data
└── working-with-claude.md         # 📚 Human documentation
```

## 📄 Example File Contents

### core/SYSTEM.md
```markdown
# Core Claude Code System Instructions

You are Claude Code, Anthropic's official CLI for Claude.

## Tone and Style
- Be concise, direct, and to the point
- Minimize output tokens while maintaining quality
- Answer with fewer than 4 lines unless asked for detail
- No unnecessary preamble or postamble

## Defensive Security
IMPORTANT: Assist with defensive security tasks only. Refuse malicious code creation.

## Code References
Include `file_path:line_number` pattern for easy navigation.

## Tool Usage
- Batch multiple tool calls for performance
- Never use cargo build/check for WASM
- Use Task tool for open-ended searches
```

### core/memory-management.md
```markdown
# Memory Management System

## Focused Productivity Entities (5 max each)
- `current_session_state` - Current work (1 item, overwrites)
- `recent_solutions` - Bug fixes to remember
- `active_blockers` - Current issues
- `daily_patterns` - Essential rules
- `next_steps` - Immediate actions

## Smart Archiving Rules
When entities reach 5 observations:
- `daily_patterns` → `comprehensive_development_patterns`
- `recent_solutions` → `comprehensive_solutions` (if important)
- `active_blockers` → `resolved_blockers` (when resolved)
- `next_steps` → `completed_tasks` (if completed)

## Automatic Updates
Update entities immediately when:
- Solving bugs → recent_solutions
- Switching tasks → current_session_state
- Finding patterns → daily_patterns
```

### frameworks/moonzoon/FRAMEWORK.md
```markdown
# MoonZoon Framework Patterns

## Architecture
- Frontend: Rust + WASM using Zoon
- Backend: Moon framework (optional)
- Build: mzoon CLI tool

## Critical Rules
- Use `zoon::println!()` not `std::println!()`
- Only mzoon handles WASM compilation properly
- Run `makers start` for development

## Project Structure
```
frontend/     - Rust/WASM frontend
backend/      - Moon backend (if enabled)
```
```

### PROJECT.md (NovyWave-specific)
```markdown
# NovyWave Project Configuration

## Project Overview
NovyWave - Professional waveform viewer for digital design verification

## Architecture
- Dual-platform: Web + Tauri desktop
- Custom UI: NovyUI component library  
- Graphics: Fast2D rendering
- Pinned MoonZoon: `7c5178d891cf4afbc2bbbe864ca63588b6c10f2a`

## Key Dependencies
- Fast2D from NovyWave/Fast2D
- IconName enum tokens (never strings)

## Development Commands
- `makers start` - Dev server at http://localhost:8080
- `makers build` - Production build
- `makers tauri` - Desktop development
```

### CLAUDE.md (Assembly)
```markdown
# CLAUDE.md

<!-- Core System Layer -->
@.claude/core/SYSTEM.md
@.claude/core/memory-management.md
@.claude/core/mcp-tools.md
@.claude/core/development-workflow.md

<!-- Framework Layer -->
@.claude/frameworks/moonzoon/FRAMEWORK.md
@.claude/frameworks/moonzoon/zoon-patterns.md
@.claude/frameworks/moonzoon/wasm-debugging.md

<!-- Project Layer -->
@.claude/PROJECT.md
@.claude/project-docs/architecture.md
@.claude/project-docs/novyui-components.md

<!-- Auto-Generated Context -->
@.claude/ai-docs/focus-context.md
```

## 🚀 Migration Examples

### New MoonZoon Project "DataViz"
```bash
# 1. Copy reusable parts
cp -r NovyWave/.claude/core DataViz/.claude/
cp -r NovyWave/.claude/frameworks DataViz/.claude/

# 2. Create PROJECT.md
cat > DataViz/.claude/PROJECT.md << EOF
# DataViz Project Configuration

## Project Overview
DataViz - Real-time data visualization dashboard

## Architecture
- Frontend: MoonZoon with custom charts
- Backend: Moon + PostgreSQL
- Real-time: WebSocket updates

## Development Commands
- \`makers start\` - Dev server at http://localhost:8080
- \`makers test\` - Run test suite
EOF

# 3. Customize commands from templates
sed 's/{{PROJECT_NAME}}/DataViz/g' .claude/frameworks/moonzoon/commands/start.template.md > .claude/commands/start.md

# 4. Create assembly CLAUDE.md (same structure)
# 5. Initialize fresh ai-memory.json
```

### New React Project "AdminPanel"
```bash
# 1. Copy only core
cp -r NovyWave/.claude/core AdminPanel/.claude/

# 2. No framework layer needed
# 3. Create custom PROJECT.md and commands
# 4. Different CLAUDE.md assembly (no MoonZoon imports)
```

## 🎉 Benefits Achieved

1. **5-minute setup** for new MoonZoon projects
2. **Consistent behavior** across all projects  
3. **Easy updates** - pull latest core improvements
4. **Clear boundaries** - know what to customize
5. **Framework agnostic** - core works everywhere