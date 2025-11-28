# macOS Installation

## Apple Silicon (M1/M2/M3)

1. Download `NovyWave_x.x.x_aarch64.dmg`
2. Open the DMG file
3. Drag NovyWave to your Applications folder

## Intel Macs

1. Download `NovyWave_x.x.x_x64.dmg`
2. Open the DMG file
3. Drag NovyWave to your Applications folder

## Gatekeeper Warning

On first launch, macOS may show a security warning because NovyWave is not notarized by Apple (beta release). To open:

### Method 1: Right-Click

1. Right-click (or Control-click) on NovyWave in Applications
2. Select "Open" from the context menu
3. Click "Open" in the dialog

### Method 2: System Preferences

1. Go to **System Preferences** → **Security & Privacy** → **General**
2. Click "Open Anyway" next to the NovyWave message

## Command Line Access

To run NovyWave from the terminal:

```bash
# Add to PATH (add to ~/.zshrc for persistence)
export PATH="/Applications/NovyWave.app/Contents/MacOS:$PATH"

# Or create an alias
alias novywave="/Applications/NovyWave.app/Contents/MacOS/NovyWave"
```

## File Associations

To associate waveform files with NovyWave:

1. Right-click a `.vcd`, `.fst`, or `.ghw` file in Finder
2. Select "Get Info"
3. Under "Open with:", select NovyWave
4. Click "Change All..." to apply to all files of that type

## Troubleshooting

### App Won't Launch

If the app crashes on launch, try running from terminal to see error messages:

```bash
/Applications/NovyWave.app/Contents/MacOS/NovyWave
```

### Performance Issues

If experiencing slow performance, ensure you're using the correct architecture version (ARM for Apple Silicon, x64 for Intel).
