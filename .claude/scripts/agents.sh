#!/bin/bash
# /agents command - Launch reusable AI agents for planning, implementation, and research

source "$(dirname "$0")/shared-functions.sh"
init_hook_env

# Parse command arguments
AGENT_TYPE="${1:-list}"
shift
TASK="$*"

# Agent launcher function
launch_agent() {
    local agent_name="$1"
    local agent_role="$2"
    local task_description="$3"
    local prompt="$4"
    
    echo "🚀 Launching $agent_name agent..."
    echo ""
    echo "Role: $agent_role"
    echo "Task: $task_description"
    echo ""
    echo "Agent Prompt:"
    echo "$prompt"
    echo ""
    echo "---"
    echo "Use Task tool to launch this agent with the above prompt."
}

# List available agents
list_agents() {
    cat << 'EOF'
# Available AI Agents

## 🧠 opus-planner
High-level system architect and planner (simulating Opus)
Usage: /agents opus-planner <task>

## 🔨 sonnet-implementor
Focused code implementor (simulating Sonnet)  
Usage: /agents sonnet-implementor <specs>

## 🔍 opus-researcher
Deep technical researcher with read-only access
Usage: /agents opus-researcher <topic>

## 📚 sonnet-researcher
Fast focused researcher with read-only access
Usage: /agents sonnet-researcher <query>

## Examples:
/agents opus-planner Design a virtual scrolling system
/agents sonnet-implementor Implement the TreeView component
/agents opus-researcher Research WASM performance patterns
/agents sonnet-researcher Find Zoon button examples

## Parallel Research:
Launch multiple researchers simultaneously for comprehensive analysis
EOF
}

case "$AGENT_TYPE" in
    list|"")
        list_agents
        ;;
        
    opus-planner)
        prompt="You are a senior architect (Opus-level depth). Analyze the codebase and create a detailed implementation plan.

Task: $TASK

Requirements:
1. Analyze existing architecture and patterns in the codebase
2. Create detailed implementation specifications with exact steps
3. Break down into concrete, actionable implementation tasks
4. Consider edge cases, error handling, and performance
5. Specify exact file modifications needed with code examples
6. Include testing strategy and verification steps

Output a structured plan that can be directly used by an implementor. Be specific about file paths, function names, and implementation details."

        launch_agent "opus-planner" "High-level system architect" "$TASK" "$prompt"
        ;;
        
    sonnet-implementor)
        prompt="You are a focused implementor (Sonnet-level efficiency). Implement the following specifications exactly as described.

Specifications/Task: $TASK

Requirements:
1. Follow the specifications or requirements precisely
2. Match existing code patterns and conventions in the codebase
3. Write clean, efficient, idiomatic code
4. Include comprehensive error handling
5. Test your implementation thoroughly
6. Verify compilation and functionality

Implement the code, test it, and verify it works correctly. Focus on getting it done efficiently."

        launch_agent "sonnet-implementor" "Focused code implementor" "$TASK" "$prompt"
        ;;
        
    opus-researcher)
        prompt="You are a deep technical researcher (Opus-level analysis). Research comprehensively without modifying any code.

Research Topic: $TASK

Requirements:
1. Analyze multiple sources, approaches, and implementations
2. Compare different solutions with detailed trade-off analysis
3. Identify best practices, patterns, and anti-patterns
4. Consider performance, maintainability, and scalability implications
5. Provide deep technical insights with concrete examples
6. Research both theoretical foundations and practical implementations

You have READ-ONLY access to the codebase. Cannot modify any files.
Return comprehensive analysis with actionable recommendations and code examples where relevant."

        launch_agent "opus-researcher" "Deep technical researcher (read-only)" "$TASK" "$prompt"
        ;;
        
    sonnet-researcher)
        prompt="You are a fast, focused researcher (Sonnet-level speed). Quickly find specific information without modifying code.

Search Target: $TASK

Requirements:
1. Find specific examples, syntax, and usage patterns quickly
2. Locate relevant documentation and references
3. Identify common implementation patterns
4. Get quick, actionable answers efficiently
5. Focus on practical, immediately usable information
6. Prioritize speed and relevance over exhaustive analysis

You have READ-ONLY access to the codebase. Cannot modify any files.
Return concise, targeted findings with specific examples."

        launch_agent "sonnet-researcher" "Fast researcher (read-only)" "$TASK" "$prompt"
        ;;
        
    parallel-research)
        echo "🔬 Launching Parallel Research Agents"
        echo ""
        echo "This will launch both opus-researcher and sonnet-researcher simultaneously."
        echo "Task: $TASK"
        echo ""
        echo "Launch these agents in parallel using Task tool:"
        echo ""
        echo "1. opus-researcher: Deep analysis of '$TASK'"
        echo "2. sonnet-researcher: Quick examples for '$TASK'"
        echo "3. sonnet-researcher: Documentation search for '$TASK'"
        ;;
        
    *)
        echo "Unknown agent: $AGENT_TYPE"
        echo ""
        list_agents
        ;;
esac

# Log agent usage
echo "$(date): Agent launched - $AGENT_TYPE: $TASK" >> "$HOOK_LOG"