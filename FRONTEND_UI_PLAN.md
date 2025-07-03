# NovyWave Frontend UI Implementation Plan

## Context
Implementing UI-only frontend for professional waveform viewer based on Figma designs in `design/figma/`. Using NovyUI components only, functionality implementation comes later.

## Target Design
- Professional 4-panel layout waveform viewer
- Dark/light theme support
- Responsive: 1440x1024 and 1920x1080
- Browser-only testing initially

## Available NovyUI Components
✅ TreeView, Button, Input, Badge, Card, List, Icon, Typography

## Implementation Phases

### Phase 1: Basic Layout Framework
- Create responsive 4-panel grid layout using Zoon
- Use Card components for panel containers  
- Test at both target resolutions

### Phase 2: Panel Implementation
**Step 1: Files & Scopes Panel (Left)**
- TreeView with mock .fst file structure
- "Load files" button (UI only)
- Search/filter input box

**Step 2: Variables Panel (Bottom-Left)** 
- Searchable list of mock variables
- Type badges (Wire 1-bit Input/Output)
- Variable selection (UI state only)

**Step 3: Selected Variables Panel (Center-Left)**
- List of selected variables with remove buttons
- "Remove All" functionality
- "Dock to Bottom" toggle
- Drag handles (visual only)

**Step 4: Waveform Panel (Center-Right)**
- Empty panel with timeline placeholder
- Zoom controls (buttons only)
- Timeline scrubber placeholder  
- Mock timing labels (10s, 20s, etc.)

### Phase 3: Theme & Polish
- Dark/light theme switching
- Responsive layout optimization
- Component styling to match designs

## Component Architecture
```rust
root()
├── app_header()           // Top bar with Load Files, Remove All
├── main_layout()          // 4-panel layout
│   ├── files_panel()      // TreeView + search
│   ├── variables_panel()  // Variable list + badges
│   ├── selected_panel()   // Selected vars + controls
│   └── waveform_panel()   // Timeline placeholder
```

## Extensions Needed
- Panel Splitter Component (resizable panels)
- Timeline Controls (zoom/scrub)
- Enhanced TreeView (file system styling)
- Variable List Item (custom with badges)

## Mock Data Strategy
- Realistic .fst file hierarchy
- Variable types and timing data
- Structure compatible with future real implementation

## Current Status
- moonzoon-novyui successfully integrated
- Working button demo with all variants
- zoon::println!() console logging working
- MCP servers configured
- Build system operational

## Next Step
Start with Phase 1: Basic Layout Framework