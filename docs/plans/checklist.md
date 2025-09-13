# NovyWave Implementation Checklist

⚠️ **IMPORTANT: Checklist Update Protocol**
- Items are marked as "In Progress" [~] when development starts
- Items are ONLY marked as "Complete" [x] after user confirmation of proper testing
- User must explicitly confirm: "Mark [item] as complete" after testing functionality
- This ensures the checklist reflects actual working functionality, not just written code
- All status changes require explicit user approval to maintain accuracy

**Progress tracking system for implementing the NovyWave waveform viewer according to the comprehensive specification.**

## Legend
- [ ] Not started
- [~] In progress
- [x] Completed
- [!] Blocked/Issues
- **[MVP]** - Required for minimum viable product
- **[v2]** - Future enhancement

---

## 📊 Overall Progress
**Architecture**: 0/12 (0%)
**UI Panels**: 0/45 (0%)
**Core Features**: 0/38 (0%)
**Performance**: 0/15 (0%)
**Total**: 0/110 (0%)

---

## 🏗️ Core Architecture

### Configuration System
- [x] **[MVP]** Development mode (.novywave file in project root)
- [ ] Production mode (Tauri user storage)
- [x] **[MVP]** TOML configuration format parsing
- [x] **[MVP]** Per-project configuration support
- [x] **[MVP]** Auto-save with debouncing (500ms panels, 1000ms timeline)
- [x] **[MVP]** Configuration error handling (corruption recovery)
- [ ] Configuration schema validation
- [x] **[MVP]** Centralized configuration management system

### Data Architecture
- [x] **[MVP]** Nanosecond (u64) internal time representation
- [x] **[MVP]** Dynamic unit formatting (ns, μs, ms, s)
- [x] **[MVP]** Multi-file time alignment system
- [ ] **[MVP]** Peak-preserving decimation algorithm
- [x] **[MVP]** Frontend-backend communication protocol
- [x] **[MVP]** Request/response data formats
- [x] **[MVP]** Batching strategy implementation
- [x] Data request optimization (incremental loading)

---

## 🖥️ Layout & UI Framework

### Core Layout
- [x] **[MVP]** 3-panel layout system (Files & Scopes, Variables, Selected Variables)
- [x] **[MVP]** Draggable divider lines with resize functionality
- [x] **[MVP]** Transparent overlay for dragging operations
- [x] **[MVP]** Dock mode switching (Right ↔ Bottom)
- [x] **[MVP]** Panel dimension persistence per dock mode
- [x] **[MVP]** Height inheritance chain (Height::fill() pattern)
- [x] Responsive width handling (Width::fill())

### Theme System
- [x] **[MVP]** Dark theme (default)
- [x] Light theme
- [x] **[MVP]** Theme toggle button (sun/moon icons)
- [x] **[MVP]** Fixed color palettes (no customization)
- [x] CSS custom properties for dynamic switching
- [x] Canvas theme integration (Fast2D colors)
- [x] **[MVP]** Ctrl+T keyboard shortcut for theme toggle

### Global Shortcuts
- [x] **[MVP]** Ctrl+D dock mode toggle
- [x] **[MVP]** Ctrl+T theme toggle
- [x] **[MVP]** Focus-based shortcut disable (when input focused)
- [ ] Modal dialog shortcut handling (Enter/Escape)
- [ ] No shortcut customization initially

---

## 📁 Files & Scopes Panel

### Header Components
- [x] **[MVP]** "Files & Scopes" title (left aligned)
- [x] **[MVP]** "Load Files" button with folder icon (center)
- [x] **[MVP]** "Remove All" button with X icon (right)
- [x] **[MVP]** Empty state placeholder text

### File Tree View
- [ ] **[MVP]** Tree view with loaded files as roots
- [ ] **[MVP]** File icons (normal/warning for parse errors)
- [ ] **[MVP]** Smart file path prefixes for disambiguation
- [ ] **[MVP]** Timespan display format (0-250s)
- [ ] **[MVP]** Individual file remove (X button)
- [ ] **[MVP]** Error tooltips on hover (with absolute paths)
- [ ] **[MVP]** Success tooltips showing absolute paths
- [ ] **[MVP]** Less contrasting colors for path prefixes
- [ ] **[MVP]** Different colors for loading errors
- [ ] **[MVP]** Less contrasting timespan text

### Scope Hierarchy
- [ ] **[MVP]** Expandable scope hierarchy from Wellen parsing
- [ ] **[MVP]** Scope chevron icons for expandable scopes
- [ ] **[MVP]** Single scope selection (checkboxes)
- [ ] **[MVP]** Scope ID format: "file|parent|scope"
- [ ] **[MVP]** Expanded scopes state persistence
- [ ] **[MVP]** Scope expansion via click (excluding checkbox)
- [ ] **[MVP]** File ordering (name then prefix path)
- [ ] **[MVP]** Scope ordering (by name)

### Session Management
- [ ] **[MVP]** Auto-reload files on startup from config
- [ ] **[MVP]** Seamless session restoration
- [ ] **[MVP]** Scroll state persistence (vertical/horizontal)
- [ ] **[MVP]** Store file paths immediately after selection

---

## 📄 Load Files Dialog

### Dialog Structure
- [x] **[MVP]** Modal with semi-transparent overlay
- [x] **[MVP]** Overlay click to close
- [x] **[MVP]** Escape key to close
- [x] **[MVP]** Cancel button
- [x] **[MVP]** Dynamic "Load N Files" button
- [x] **[MVP]** Button disabled until file selected

### Dialog Header
- [x] **[MVP]** "Select Waveform Files" title
- [x] **[MVP]** Less contrasting "(*.vcd, *.fst, *.ghw)" subtitle
- [ ] Optional X button to close

### File Tree Navigation
- [x] **[MVP]** Standard file system tree view
- [x] **[MVP]** Directory expansion/collapse
- [x] **[MVP]** File filtering (VCD, FST, GHW only)
- [x] **[MVP]** Multiple file selection via checkboxes
- [x] **[MVP]** File highlighting on selection
- [x] **[MVP]** Tree starts from root (/) on Linux

### Directory State Management
- [x] **[MVP]** Directory expansion state persistence
- [x] **[MVP]** Default home directory expansion on first use
- [x] **[MVP]** Ancestor directory expansion for home
- [x] **[MVP]** Symbolic link transparent handling
- [x] **[MVP]** Inaccessible symlink error handling

### Placeholder Messages
- [x] **[MVP]** Empty directory placeholder ("Empty")
- [x] **[MVP]** No supported files placeholder ("No supported files")
- [x] **[MVP]** Permission error placeholder ("Can't access this directory")
- [x] **[MVP]** Contrasting colors for error placeholders

### Selected Files Display
- [x] **[MVP]** Selected files as pills/tags below tree
- [ ] **[MVP]** X button on pills to deselect
- [ ] **[MVP]** Automatic tree view unchecking
- [ ] **[MVP]** Tooltip with absolute path on pill hover

### Scroll Position
- [ ] Scroll position persistence in config
- [ ] **[MVP]** Debounced scroll position saving (500ms)
- [ ] Scroll restoration timing coordination
- [ ] **[MVP]** Wait for tree structure before scroll restore

---

## 📊 Variables Panel

### Header Components
- [x] **[MVP]** "Variables" title (left aligned)
- [x] **[MVP]** Variable count display (less contrasting)
- [x] **[MVP]** Search/filter input with magnifying glass icon
- [x] **[MVP]** X icon in search input for clearing
- [x] **[MVP]** Real-time filtering on input change
- [x] **[MVP]** Case-insensitive substring filtering
- [x] **[MVP]** Filtered results count update

### Virtual List Implementation
- [x] **[MVP]** Virtual list for 5,000+ variables
- [x] **[MVP]** Constant row height for performance
- [x] **[MVP]** No text wrapping (horizontal scrollbar)
- [x] **[MVP]** Stable scrolling performance
- [x] **[MVP]** Search filtering integration with virtual list
- [x] **[MVP]** Memory efficient (visible items + buffer only)
- [x] **[MVP]** 60fps scroll responsiveness
- [x] Selection state persistence across virtual recycling

### Variable Display
- [x] **[MVP]** Variable name (left aligned)
- [x] **[MVP]** Variable type (right aligned, e.g., "Wire 1-bit")
- [ ] **[MVP]** Smaller font and blue-ish color for types
- [ ] **[MVP]** Common prefix graying (AB, (AB)C, (ABC)D)
- [ ] **[MVP]** Hover highlighting
- [ ] **[MVP]** Selection highlighting
- [ ] **[MVP]** Multi-selection support (entire row click)
- [ ] **[MVP]** Variable sorting by name
- [ ] **[MVP]** Pre-selection from config on startup

### Empty States
- [ ] **[MVP]** "Selected scope does not have any variables" placeholder
- [ ] **[MVP]** "Select scope in the Files & Scopes panel" placeholder
- [ ] **[MVP]** Standardized placeholder styling

---

## ⚡ Selected Variables Panel

### Header Components
- [x] **[MVP]** "Selected Variables" title (left aligned)
- [x] **[MVP]** Theme toggle button (sun/moon, no text)
- [x] **[MVP]** Dock mode toggle button with icon and text
- [x] **[MVP]** "Remove All" button (right aligned)
- [x] **[MVP]** Button text updates based on dock mode
- [x] **[MVP]** Icon updates based on dock mode

### Column Structure
- [x] **[MVP]** Three-column layout (Name, Value, Wave)
- [x] **[MVP]** Draggable dividers between columns
- [x] **[MVP]** Column width persistence per dock mode
- [x] **[MVP]** Consistent row height across all columns
- [x] **[MVP]** Table-like alignment

### Name Column
- [x] **[MVP]** Remove X button per variable
- [x] **[MVP]** Variable name display
- [x] **[MVP]** Variable type display (same styling as Variables panel)
- [x] **[MVP]** Tooltip with variable ID on hover (file|scope|path)

### Name Column Footer
- [x] **[MVP]** Keyboard shortcut indicators [Z] [W] [S] [R]
- [x] **[MVP]** Current zoom display (15ns/px format)
- [x] **[MVP]** Tooltips for all shortcut keys
- [x] **[MVP]** Dynamic zoom unit formatting

### Value Column
- [ ] **[MVP]** Dropdown/Select element per variable
- [ ] **[MVP]** Formatted value display (left aligned)
- [ ] **[MVP]** Copy to clipboard button (copy icon)
- [ ] **[MVP]** Formatter name display (Hex, Bin, etc.)
- [ ] **[MVP]** Chevron icon for dropdown indication
- [ ] **[MVP]** Click prevention on copy button

### Value Column Formatters
- [ ] **[MVP]** Hexadecimal formatter (default)
- [ ] **[MVP]** Binary formatter
- [ ] **[MVP]** ASCII/Text formatter
- [ ] **[MVP]** BinaryWithGroups formatter (4-bit spacing)
- [ ] **[MVP]** Octal formatter
- [ ] **[MVP]** Signed integer formatter
- [ ] **[MVP]** Unsigned integer formatter
- [ ] **[MVP]** Formatter persistence in config
- [ ] **[MVP]** Dropdown menu for formatter selection
- [ ] **[MVP]** Formatter preview in dropdown options

### Value Column Footer
- [ ] **[MVP]** Timeline boundaries display (0s, 250s)
- [ ] **[MVP]** Cursor position display (125s)
- [ ] **[MVP]** Keyboard shortcut indicators [A] [Q] [E] [D]
- [ ] **[MVP]** Tooltips for panning and cursor keys
- [ ] **[MVP]** Dynamic boundary calculation
- [ ] **[MVP]** Auto-refresh on file set changes

### Wave Column (Canvas)
- [ ] **[MVP]** Fast2D canvas integration
- [ ] **[MVP]** Signal transition blocks with values
- [ ] **[MVP]** Alternating row backgrounds for distinction
- [ ] **[MVP]** Timeline footer with ticks
- [ ] **[MVP]** Dynamic tick spacing based on zoom
- [ ] **[MVP]** Edge numbers (left/right boundaries)
- [ ] **[MVP]** Tick marks above numbers
- [ ] **[MVP]** Yellow cursor line (vertical)
- [ ] **[MVP]** Purple zoom center line (dashed, vertical)

### Wave Column Interactions
- [ ] **[MVP]** Mouse click to move cursor
- [ ] **[MVP]** Mouse hover to move zoom center
- [ ] **[MVP]** Zoom center default at left boundary (0)
- [ ] **[MVP]** Canvas resize handling
- [ ] **[MVP]** Smooth visual transitions

---

## ⌨️ Keyboard Shortcuts System

### Timeline Navigation
- [x] **[MVP]** Z key - Move zoom center to 0
- [x] **[MVP]** R key - Reset to default state (full reset)
- [x] **[MVP]** W key - Zoom in (centered on zoom center)
- [ ] **[MVP]** Shift+W - Zoom in faster (3-5x acceleration)
- [x] **[MVP]** S key - Zoom out (centered on zoom center)
- [ ] **[MVP]** Shift+S - Zoom out faster (3-5x acceleration)

### Cursor Movement
- [x] **[MVP]** Q key - Move cursor left continuously
- [x] **[MVP]** Shift+Q - Jump to previous signal transition
- [x] **[MVP]** E key - Move cursor right continuously
- [x] **[MVP]** Shift+E - Jump to next signal transition
- [x] **[MVP]** Smooth cursor movement when holding keys

### Viewport Panning
- [x] **[MVP]** A key - Pan timeline left
- [ ] **[MVP]** Shift+A - Pan timeline left faster (2-3x)
- [x] **[MVP]** D key - Pan timeline right
- [ ] **[MVP]** Shift+D - Pan timeline right faster (2-3x)
- [x] **[MVP]** Smooth panning when holding keys

### Focus Management
- [x] **[MVP]** Disable all shortcuts when filter input focused
- [x] **[MVP]** Clear focus outline indication
- [x] **[MVP]** Theme/dock shortcuts work in modals
- [x] **[MVP]** Timeline navigation works in modals

---

## 🎨 Special Signal States

### State Support
- [ ] **[MVP]** High-Impedance (Z) state handling
- [ ] **[MVP]** Unknown (X) state handling
- [ ] **[MVP]** Uninitialized (U) state handling
- [ ] **[MVP]** No Data Available (N/A) state handling

### Value Column Display
- [ ] **[MVP]** Z state as "Z" with contrasting color
- [ ] **[MVP]** X state as "X" with contrasting color
- [ ] **[MVP]** U state as "U" with contrasting color
- [ ] **[MVP]** N/A state as "N/A" with low contrast

### Wave Column Display
- [ ] **[MVP]** Z state as gray/yellow mid-level block
- [ ] **[MVP]** X state as red full-height block
- [ ] **[MVP]** U state as red block (same as X)
- [ ] **[MVP]** N/A state as gap (no block)

### Formatter Behavior
- [ ] **[MVP]** Binary formatter: Z→Z, X→X, U→U, N/A→N/A
- [ ] **[MVP]** Hex formatter: Z→Z, X→X, U→?, N/A→N/A
- [ ] **[MVP]** Decimal formatters: Z→-, X→-, U→-, N/A→N/A
- [ ] **[MVP]** ASCII formatter: Z→., X→., U→., N/A→N/A

### Educational Tooltips
- [ ] **[MVP]** Z state tooltip with explanation
- [ ] **[MVP]** X state tooltip with explanation
- [ ] **[MVP]** U state tooltip with explanation
- [ ] **[MVP]** N/A state tooltip with explanation
- [ ] **[MVP]** HTML tooltip formatting with examples

---

## 🔄 Data Loading & Performance

### Loading Strategy
- [ ] **[MVP]** Wellen header-only parsing (fast)
- [ ] **[MVP]** On-demand full signal data loading
- [ ] **[MVP]** Loading indicator for >500ms operations
- [ ] **[MVP]** Time span and units extraction
- [ ] **[MVP]** Error handling for parsing failures

### Request Optimization
- [ ] **[MVP]** Batch requests for all selected variables
- [ ] **[MVP]** Incremental loading for single variable additions
- [ ] **[MVP]** Bundle requests for timeline range changes
- [ ] **[MVP]** Bundle requests for cursor outside visible range
- [ ] **[MVP]** Request throttling during smooth operations
- [ ] Backend multithreading optimization
- [ ] Frontend memory usage minimization

### Decimation Implementation
- [ ] **[MVP]** Pixel-aligned bucket algorithm
- [ ] **[MVP]** Min/max value preservation per bucket
- [ ] **[MVP]** First/last transition time preservation
- [ ] **[MVP]** Single-pixel pulse visibility (minimum 1px blocks)
- [ ] **[MVP]** Setup/hold violation preservation
- [ ] **[MVP]** Bright contrasting colors for critical events

### Performance Targets
- [ ] **[MVP]** 60fps panning/zooming
- [ ] **[MVP]** Real-time cursor updates (Q/E hold)
- [ ] **[MVP]** Smooth mixed timescale handling (simple.vcd + wave_27.fst)
- [ ] **[MVP]** Dynamic timeline tick formatting without layout breaks
- [ ] **[MVP]** Virtual list 60fps scrolling
- [ ] **[MVP]** Handle 10,000+ transitions per visible signal

---

## 🚨 Error Handling System

### Toast Notifications
- [ ] **[MVP]** Toast popups from top-right corner
- [ ] **[MVP]** Red panels with thin border and shadows
- [ ] **[MVP]** Warning icon, title, message, X button
- [ ] **[MVP]** Stacking with vertical offset
- [ ] **[MVP]** Display in front of all elements (including modals)

### Auto-Dismiss Behavior
- [ ] **[MVP]** Progress bar integrated to bottom edge
- [ ] **[MVP]** Leftward emptying animation
- [ ] **[MVP]** Configurable duration (toast_auto_dismiss_ms)
- [ ] **[MVP]** Default 5000ms (5 seconds)
- [ ] **[MVP]** Click to pause/resume functionality
- [ ] **[MVP]** Manual close via X button
- [ ] **[MVP]** Tooltip help on hover

### No-Retry Policy
- [ ] **[MVP]** No automatic file load retry
- [ ] **[MVP]** No automatic config corruption retry
- [ ] **[MVP]** No automatic network error retry
- [ ] **[MVP]** Clear user messaging for all errors
- [ ] **[MVP]** Manual recovery instructions

### Contextual Error Display
- [ ] **[MVP]** File tree warning icons for parse failures
- [ ] **[MVP]** Different colors for loading errors
- [ ] **[MVP]** Directory permission error placeholders
- [ ] **[MVP]** Tooltip error details with file paths
- [ ] **[MVP]** Value column special state indicators

### Error Logging
- [ ] **[MVP]** Browser console for development debugging
- [ ] Backend logs for server-side errors
- [ ] **[MVP]** User notifications through UI

---

## 🧪 Implementation Requirements

### Virtual List Tech Details
- [ ] **[MVP]** Element pool stability (update content, not DOM)
- [ ] **[MVP]** Velocity-based buffering (5-15 elements)
- [ ] **[MVP]** Selection state across recycling
- [ ] **[MVP]** Real-time filtered results support

### Timeline Precision
- [ ] **[MVP]** u64 nanosecond arithmetic throughout
- [ ] **[MVP]** No floating-point precision loss
- [ ] **[MVP]** Mouse-to-timeline coordinate conversion
- [ ] **[MVP]** Zoom center fixed nanosecond positioning

### Memory Management
- [ ] **[MVP]** Frontend minimal working set
- [ ] **[MVP]** Backend large dataset operations
- [ ] **[MVP]** No cached formatted values
- [ ] **[MVP]** Real-time value formatting

### Cross-Platform Support
- [ ] **[MVP]** Development mode (.novywave in project root)
- [ ] Production mode (Tauri platform storage)
- [ ] **[MVP]** File path separator handling (/, \)
- [ ] **[MVP]** Keyboard shortcut compatibility
- [ ] **[MVP]** File permission handling

---

## 🎯 Testing & Validation

### Performance Testing
- [ ] **[MVP]** Mixed timescale testing (simple.vcd + wave_27.fst)
- [ ] Large file testing (multi-gigabyte waveforms)
- [ ] **[MVP]** 10,000+ variable Variables panel testing
- [ ] Memory usage monitoring during extended use
- [ ] **[MVP]** 60fps timeline rendering validation

### Functional Testing
- [ ] **[MVP]** Session restoration testing
- [ ] **[MVP]** Configuration persistence testing
- [ ] **[MVP]** All keyboard shortcuts testing
- [ ] **[MVP]** Error handling scenario testing
- [ ] **[MVP]** Special signal state display testing

### Integration Testing
- [ ] **[MVP]** File loading workflow testing
- [ ] **[MVP]** Variable selection workflow testing
- [ ] **[MVP]** Timeline navigation workflow testing
- [ ] **[MVP]** Theme switching testing
- [ ] **[MVP]** Dock mode switching testing

---

## 📈 Progress Tracking

### Current Sprint Focus
- [ ] **Current**:
- [ ] **Next**:
- [ ] **Blocked**:

### Milestone Targets
- [ ] **MVP Release**: All [MVP] items completed
- [ ] **Performance Baseline**: 60fps + smooth operations
- [ ] **Full Feature Set**: All non-[v2] items completed
- [ ] **Polish & Optimization**: All remaining items

### Notes Section
```
Implementation notes, decisions, and blockers:

[Add notes as development progresses]
```

---

**Last Updated**: [Date]
**Items Completed**: 0/110
**MVP Progress**: 0% (0 of X MVP items)