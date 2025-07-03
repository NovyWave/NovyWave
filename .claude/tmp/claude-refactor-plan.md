# Claude Configuration Modular Refactor Plan

## 🎯 Goal
Create a modular, reusable Claude Code configuration system that can be easily migrated between projects with minimal changes.

## 📊 Proposed Three-Tier Architecture

### 1. **Core System Layer** (Universal)
Transferable to ANY project using Claude Code.

```
.claude/core/
├── SYSTEM.md                 # Core Claude behavior, tone, general rules
├── memory-management.md      # Memory MCP patterns, focus system, archiving
├── browser-automation.md     # Browser MCP usage patterns
├── development-workflow.md   # General dev practices, testing, git
├── commands/
│   ├── focus.md             # Display productivity context
│   ├── note.md              # Store discoveries with archiving
│   ├── memory-search.md     # Search Memory MCP
│   └── memory-cleanup.md    # Monthly maintenance
└── hooks/
    └── memory-context-sync.sh # Auto-update focus context
```

### 2. **Framework Layer** (MoonZoon-specific)
Transferable to any MoonZoon-based project.

```
.claude/frameworks/moonzoon/
├── FRAMEWORK.md              # MoonZoon setup, architecture
├── zoon-patterns.md          # Zoon UI framework patterns
├── moon-backend.md           # Moon backend patterns
├── wasm-debugging.md         # WASM compilation, mzoon workflows
└── commands/
    ├── start.template.md     # Template for makers start
    └── stop.template.md      # Template for makers stop
```

### 3. **Project Layer** (NovyWave-specific)
Only this needs rewriting for new projects.

```
.claude/
├── PROJECT.md                # NovyWave architecture, goals, structure
├── project-docs/
│   ├── novyui-components.md  # Custom UI library patterns
│   ├── waveform-domain.md    # Domain-specific rules
│   └── fast2d-graphics.md    # Graphics library usage
└── project-commands/
    ├── start.md              # Customized from template
    └── stop.md               # Customized from template
```

### 4. **Assembly & Generated Files**

```
.claude/
├── CLAUDE.md                 # Main file that imports all layers
├── ai-docs/
│   └── focus-context.md      # Auto-generated, never edit
├── ai-memory.json            # Project-specific memory
├── settings.json             # Hooks configuration
└── working-with-claude.md    # Human guide
```

## 📋 Migration Strategy

### For New MoonZoon Project:
1. Copy entire `.claude/core/` directory
2. Copy entire `.claude/frameworks/moonzoon/` directory
3. Create new `PROJECT.md` with project specifics
4. Customize command templates for project
5. Update `CLAUDE.md` imports
6. Initialize fresh `ai-memory.json`

### For New Non-MoonZoon Project:
1. Copy entire `.claude/core/` directory
2. Skip framework layer or add different framework
3. Create new `PROJECT.md` with project specifics
4. Create project-specific commands
5. Update `CLAUDE.md` imports
6. Initialize fresh `ai-memory.json`

## 🔧 CLAUDE.md Structure (Assembly)

```markdown
# CLAUDE.md

<!-- Core System Instructions -->
@.claude/core/SYSTEM.md
@.claude/core/memory-management.md
@.claude/core/development-workflow.md

<!-- Framework-Specific Patterns (if applicable) -->
@.claude/frameworks/moonzoon/FRAMEWORK.md
@.claude/frameworks/moonzoon/zoon-patterns.md
@.claude/frameworks/moonzoon/wasm-debugging.md

<!-- Project-Specific Instructions -->
@.claude/PROJECT.md
@.claude/project-docs/novyui-components.md
@.claude/project-docs/waveform-domain.md

<!-- Auto-Generated Focus Context -->
@.claude/ai-docs/focus-context.md
```

## 🚀 Implementation Steps

### Phase 1: Extract Core System
1. Move general Claude instructions to `core/SYSTEM.md`
2. Extract memory management patterns to `core/memory-management.md`
3. Move universal commands to `core/commands/`
4. Make hooks project-agnostic

### Phase 2: Extract Framework Layer
1. Move MoonZoon patterns to `frameworks/moonzoon/`
2. Create command templates with placeholders
3. Document framework-specific workflows

### Phase 3: Isolate Project Layer
1. Keep only NovyWave-specific content in `PROJECT.md`
2. Move domain rules to `project-docs/`
3. Customize command templates

### Phase 4: Update Assembly
1. Rewrite `CLAUDE.md` as simple import list
2. Update `working-with-claude.md` with new structure
3. Test complete system

## 💡 Benefits

1. **Modularity**: Clear separation of concerns
2. **Reusability**: Core and framework layers transfer easily
3. **Maintainability**: Updates to core benefit all projects
4. **Clarity**: Easy to see what's project-specific
5. **Scalability**: Can add more frameworks easily

## 🎯 Success Criteria

- [ ] New MoonZoon project setup < 5 minutes
- [ ] New non-MoonZoon project setup < 10 minutes
- [ ] Core updates can be pulled into existing projects
- [ ] Framework patterns shared across projects
- [ ] Project-specific code clearly isolated

## 📝 Alternative Considerations

### Option B: Template Repository
- Create `claude-code-template` repo
- Use git submodules for core/framework
- Project customizes only PROJECT.md

### Option C: Configuration Generator
- Interactive setup script
- Generates customized .claude folder
- Templates with project variables

## 🤔 Questions to Resolve

1. Should commands be YAML-configured for easier templating?
2. How to handle project-specific Memory MCP entities?
3. Should we version the core/framework layers?
4. Best way to distribute updates to core?

## 📊 Current → Target Comparison

**Current**: Monolithic, project-specific, hard to migrate
**Target**: Modular, reusable, 5-minute setup for new projects

This plan prioritizes ease of migration while maintaining all current functionality. The three-tier system (Core → Framework → Project) provides the right balance of reusability and customization.