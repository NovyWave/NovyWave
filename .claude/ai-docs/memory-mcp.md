# Memory Server (@modelcontextprotocol/server-memory)

**Purpose:** Persistent knowledge graph storage across Claude Code sessions  
**Storage:** `novywave/ai-memory.json` (tracked in git for team knowledge sharing)

## Knowledge Graph Structure

- **Entities**: Primary nodes (e.g., "NovyWave project", "Martin Kavik", "Fast2D library")
- **Relations**: Directed connections in active voice (e.g., "Martin Kavik" -> "maintains" -> "NovyWave project")
- **Observations**: Facts about entities (e.g., development preferences, architectural decisions)

## Usage Patterns

- **Project Memory**: Architecture decisions, performance considerations, common issues/solutions
- **Developer Context**: Workflow preferences, coding patterns, testing approaches
- **Progress Tracking**: Milestones, feature requests, optimization targets
- **Team Knowledge**: Shared understanding of components, integration patterns

## Memory Categories

- **Identity**: Key people (developers, contributors, stakeholders)
- **Technical**: Framework choices, library decisions, build system preferences
- **Behavioral**: Development patterns, testing approaches, code style preferences
- **Goals**: Project objectives, feature roadmap, performance targets
- **Relationships**: Dependencies between components, integration patterns

## Best Practices

- Always begin sessions by retrieving relevant memory context
- Create entities for major components, people, and concepts
- Use active voice for relations ("uses", "depends on", "implements")
- Add observations for specific facts, decisions, and preferences
- Maintain 3 degrees of separation for relationships