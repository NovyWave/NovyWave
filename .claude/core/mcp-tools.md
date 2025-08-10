# MCP Tools Usage Patterns


## Browser MCP (@browsermcp/mcp)

**Purpose:** Browser automation for web research, testing, and interaction

### Common Use Cases
- **Research**: Fetch documentation, check library updates, browse examples
- **Testing**: Verify deployed applications, check responsive design
- **Integration**: Test API endpoints through web interfaces
- **Documentation**: Capture screenshots, verify examples work

### Usage Guidelines
- Automation happens locally for privacy and speed
- Uses real browser fingerprint to avoid CAPTCHAs
- Access to logged-in sessions (GitHub, npm, documentation sites)
- Can navigate, fill forms, click elements, take screenshots

### Security Notes
- Browser actions are performed in your actual browser profile
- Be cautious with form submissions and account changes
- Review automation scripts before execution
- Consider using separate browser profile for automation if needed

### Common Commands
- `mcp__browsermcp__browser_navigate` - Go to URL
- `mcp__browsermcp__browser_snapshot` - Get page accessibility tree
- `mcp__browsermcp__browser_click` - Click element
- `mcp__browsermcp__browser_type` - Type text
- `mcp__browsermcp__browser_screenshot` - Capture page

## Tool Usage Best Practices

### Batching
- Call multiple tools in a single response for optimal performance
- Run parallel bash commands with multiple tool calls

### Search Strategy
- Use Task tool for open-ended searches to reduce context usage
- Prefer Glob/Grep for specific file/pattern searches
- Use Agent tool for complex multi-step searches

### File Operations
- Always read files before editing
- Use MultiEdit for multiple changes to same file
- Prefer editing existing files over creating new ones