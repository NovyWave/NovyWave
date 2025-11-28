# Windows Installation

## Installer

1. Download `NovyWave_x.x.x_x64-setup.exe`
2. Run the installer
3. Follow the installation wizard
4. Launch NovyWave from the Start Menu

## Portable Version

For a portable installation (no admin rights required):

1. Download `NovyWave_x.x.x_x64.zip`
2. Extract to your preferred location
3. Run `NovyWave.exe`

## Windows Defender SmartScreen

On first launch, Windows may show a SmartScreen warning because NovyWave is not code-signed (beta release):

1. Click "More info"
2. Click "Run anyway"

## Adding to PATH

To run NovyWave from Command Prompt or PowerShell:

1. Open **System Properties** → **Advanced** → **Environment Variables**
2. Under "User variables", edit "Path"
3. Add the NovyWave installation directory (e.g., `C:\Program Files\NovyWave`)

Or in PowerShell:

```powershell
$env:Path += ";C:\Program Files\NovyWave"
```

## File Associations

The installer automatically creates file associations for `.vcd`, `.fst`, and `.ghw` files. To manually set associations:

1. Right-click a waveform file
2. Select "Open with" → "Choose another app"
3. Click "More apps" → "Look for another app on this PC"
4. Navigate to `NovyWave.exe`
5. Check "Always use this app"

## WebView2 Runtime

NovyWave requires the Microsoft Edge WebView2 Runtime. This is usually pre-installed on Windows 10/11. If you see an error about WebView2:

1. Download from [Microsoft WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
2. Install the Evergreen Standalone Installer

## Troubleshooting

### White Screen on Launch

If you see a white screen instead of the UI:

1. Ensure WebView2 Runtime is installed
2. Try running as Administrator
3. Check for antivirus interference

### Missing DLL Errors

Install the [Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) if you see DLL errors.
