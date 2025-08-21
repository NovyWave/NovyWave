# Agent Memory Safety Guidelines

## CRITICAL: Node.js Heap Exhaustion Issue

**Problem**: Running 3+ subagents simultaneously causes:
```
FATAL ERROR: Reached heap limit Allocation failed - JavaScript heap out of memory
```

## Agent Usage Limits

### Main Session Rules
- **Maximum 1-2 agents total per complex task**
- **NEVER run agents in parallel unless absolutely necessary**
- **Prefer single comprehensive agent over multiple specialized ones**

### Agent-to-Agent Delegation Rules
- **Planner**: Maximum 1 research agent at a time
- **Implementor**: NEVER delegate to other agents - work directly
- **Validator**: NEVER delegate to other agents - work directly  
- **Researcher**: Can be the single delegated agent from planner

### Safe Usage Patterns
```
✅ SAFE: Main → Planner → Researcher (sequential)
✅ SAFE: Main → Implementor → Validator (sequential) 
✅ SAFE: Main → Single Researcher

❌ UNSAFE: Main → 3+ agents simultaneously
❌ UNSAFE: Planner → Multiple researchers
❌ UNSAFE: Recursive delegation chains
```

### Recovery Procedure
If you encounter memory crashes:
1. Restart claude session completely
2. Use direct tool calls instead of agents
3. Break complex tasks into smaller sessions
4. Avoid multi-agent orchestration

## Implementation Strategy for Complex Tasks

Instead of multi-agent approaches, use:
1. **Sequential agent usage** (one completes before next starts)
2. **Direct tool usage** in main session when possible
3. **Focused single-purpose agents** rather than orchestration
4. **Session breaks** for very complex multi-step work

## Monitoring
Watch for these warning signs:
- Slow response times
- Long pauses during agent execution
- Memory usage climbing rapidly
- Previous session crashes with heap errors

**When in doubt, use fewer agents and more direct tool calls.**