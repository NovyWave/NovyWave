# Keyboard Shortcuts Reference

NovyWave provides comprehensive keyboard shortcuts for efficient waveform navigation and analysis.

## Global Shortcuts

### Theme and Layout
- **Ctrl+T**: Toggle between dark and light theme
- **Ctrl+D**: Toggle dock mode (Right ↔ Bottom)

### Timeline Navigation
- **Z**: Move zoom center to position 0 (timeline start)
- **R**: Reset to default state (zoom center to 0, cursor to center, full timeline visible)

### Zoom Controls
- **W**: Zoom in (centered on zoom center position)
- **Shift+W**: Zoom in faster (accelerated zoom)
- **S**: Zoom out (centered on zoom center position)
- **Shift+S**: Zoom out faster (accelerated zoom)

### Cursor Movement
- **Q**: Move timeline cursor left continuously
- **Shift+Q**: Jump cursor to previous signal transition
- **E**: Move timeline cursor right continuously
- **Shift+E**: Jump cursor to next signal transition

### Viewport Panning
- **A**: Pan timeline view left
- **Shift+A**: Pan timeline view left faster
- **D**: Pan timeline view right
- **Shift+D**: Pan timeline view right faster

## Modal Dialog Shortcuts

### File Selection Dialog
- **Enter**: Confirm selection and load files (equivalent to "Load N Files" button)
- **Escape**: Close dialog without changes (equivalent to Cancel button)

## Focus-Based Behavior

### Input Focus Handling
- **When variable filter input is focused**: All navigation shortcuts are disabled
- **When no input has focus**: All shortcuts are active and responsive
- **Focus indication**: Input fields show clear focus outline when active

### Modal Dialog Behavior
- **Dialog open**: Theme toggle (Ctrl+T) remains active for user convenience
- **Dialog open**: Dock mode toggle (Ctrl+D) remains active
- **Dialog open**: Timeline navigation shortcuts remain functional
- **Rationale**: Allows theme switching and quick navigation even during file selection

## Shortcut Customization

- **Initial implementation**: No shortcut customization available
- **Fixed shortcuts**: All shortcuts use predefined key combinations
- **Future consideration**: Customization may be added based on user feedback

## Smooth Interaction Requirements

### Continuous Movement
- **Holding Q/E keys**: Smooth cursor movement across timeline
- **Holding A/D keys**: Smooth viewport panning left/right
- **Performance target**: Responsive feel with mixed timescales (simple.vcd + wave_27.fst)

### Accelerated Actions
- **Shift modifiers**: Provide faster movement/zoom for power users
- **Zoom acceleration**: 3-5x faster zoom speed with Shift+W/S
- **Pan acceleration**: 2-3x faster panning with Shift+A/D
- **Cursor jump**: Jump to exact transition points with Shift+Q/E

## Tooltip Documentation

### Footer Tooltips
Navigation keys display helpful tooltips when hovered:

- **[Z]**: "Press Z to move zoom center to 0"
- **[W]**: "Press W to zoom in. Press Shift+W to zoom in faster."
- **[S]**: "Press S to zoom out. Press Shift+S to zoom out faster."
- **[R]**: "Press R to reset to default zoom center, zoom and cursor position."
- **[Q]**: "Press Q to move cursor left. Press Shift+Q to jump to the previous transition."
- **[E]**: "Press E to move cursor right. Press Shift+E to jump to the next transition."
- **[A]**: "Press A to pan left. Press Shift+A to pan faster."
- **[D]**: "Press D to pan right. Press Shift+D to pan faster."
