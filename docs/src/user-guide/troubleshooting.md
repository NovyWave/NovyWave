# Troubleshooting

Solutions for common issues when using NovyWave.

## Application Won't Start

### White Screen on Launch

**Symptoms:** Application window opens but shows only white/blank content.

**Solutions:**
1. **Windows:** Ensure WebView2 runtime is installed
2. **Linux:** Install WebKit dependencies:
   ```bash
   sudo apt-get install libwebkit2gtk-4.1-0
   ```
3. Check antivirus isn't blocking the application

### Crash on Startup

**Possible causes:**
- Corrupted configuration file
- Missing system libraries

**Solutions:**
1. Delete configuration file and restart:
   ```bash
   rm ~/.config/novywave/.novywave  # Linux
   ```
2. Run from terminal to see error messages:
   ```bash
   novywave
   ```

## File Loading Issues

### "Unsupported File Format"

**Cause:** File extension not recognized or file is corrupted.

**Solutions:**
1. Ensure file has correct extension (`.vcd`, `.fst`, `.ghw`)
2. Verify file isn't truncated (check file size)
3. Try opening in another viewer to verify file integrity

### File Loads but Shows No Signals

**Possible causes:**
- No scope selected
- Wrong scope selected (empty scope)

**Solutions:**
1. Expand the file tree in Files & Scopes panel
2. Click checkbox next to a scope with signals
3. Use search in Variables panel to find specific signals

### "Loading..." Never Completes

**Possible causes:**
- Very large file
- File parsing error

**Solutions:**
1. Check terminal for error messages
2. Try a smaller file to verify NovyWave works
3. Convert VCD to FST for faster loading

## Display Issues

### Signals Show "N/A"

**Cause:** Cursor is outside the file's time range.

**Solutions:**
1. Press `R` to reset view to full timeline
2. Check file's time span in file tree (shown in parentheses)
3. Move cursor within the file's time range

### Values Show "X" or "Z"

**Cause:** These are valid signal states, not errors.

- **X** = Unknown/undefined value
- **Z** = High-impedance (floating)
- **U** = Uninitialized

These typically occur at simulation start or with tri-state signals.

### Waveform Display is Blank

**Possible causes:**
- No variables selected
- Zoom level too extreme

**Solutions:**
1. Check Variables panel - are any signals clicked/highlighted?
2. Press `R` to reset zoom to full timeline
3. Check cursor position is within file's time range

## Performance Issues

### Slow Loading

**For large files:**
1. Consider using FST format instead of VCD
2. FST files are 10-100x smaller and faster to load

### Sluggish Navigation

**Solutions:**
1. Reduce number of selected variables
2. Close unused waveform files
3. Check system resource usage

## Configuration Problems

### Settings Not Saving

**Possible causes:**
- Permission issues
- Disk full
- Config directory doesn't exist

**Solutions:**
1. Check config directory exists and is writable
2. Verify disk space
3. Look for error messages in terminal

### Wrong Settings Restored

**Cause:** Multiple config files in different locations.

**Solutions:**
1. Check for `.novywave` in current directory
2. Remove project-local config to use global settings:
   ```bash
   rm .novywave  # In project directory
   ```

## Keyboard Shortcuts Not Working

### No Response to Keys

**Possible causes:**
- Input field has focus
- Dialog is open

**Solutions:**
1. Click on waveform area to defocus inputs
2. Close any open dialogs
3. Ensure NovyWave window has focus

### Wrong Key Behavior

**Cause:** OS keyboard settings or shortcuts conflicting.

**Solutions:**
1. Check for conflicting system shortcuts
2. Try on different keyboard layout

## Getting Help

If these solutions don't resolve your issue:

1. **Check GitHub Issues:** [NovyWave Issues](https://github.com/NovyWave/NovyWave/issues)
2. **Open New Issue:** Include:
   - NovyWave version
   - Operating system
   - Steps to reproduce
   - Error messages from terminal
3. **Discussions:** [GitHub Discussions](https://github.com/NovyWave/NovyWave/discussions) for questions
