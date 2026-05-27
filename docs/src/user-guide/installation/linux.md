# Linux Installation

## Debian/Ubuntu (.deb)

Download the `.deb` package and install with:

```bash
sudo dpkg -i novywave_x.x.x_amd64.deb
```

If you encounter dependency issues:

```bash
sudo apt-get install -f
```

## AppImage (Universal)

The AppImage works on most Linux distributions without installation:

```bash
chmod +x NovyWave_x.x.x_amd64.AppImage
./NovyWave_x.x.x_amd64.AppImage
```

### Making AppImage Accessible System-Wide

```bash
# Move to applications directory
sudo mv NovyWave_x.x.x_amd64.AppImage /usr/local/bin/novywave

# Create desktop entry (optional)
cat > ~/.local/share/applications/novywave.desktop << EOF
[Desktop Entry]
Name=NovyWave
Exec=/usr/local/bin/novywave
Type=Application
Categories=Development;Electronics;
EOF
```

## Dependencies

NovyWave requires the following system libraries (usually pre-installed):

- `libwebkit2gtk-4.1` - WebView rendering
- `libgtk-3-0` - GTK+ 3 runtime
- `libssl3` - OpenSSL

On Debian/Ubuntu, install with:

```bash
sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0 libssl3
```

## Wayland vs X11

NovyWave works with both Wayland and X11. If you experience issues on Wayland, try running with:

```bash
GDK_BACKEND=x11 novywave
```

## WebKit Rendering Profiles

The Linux desktop app uses WebKitGTK. If rendering is slow or the window is blank, try these profiles:

```bash
# Default: hardware acceleration enabled, no forced WebKit environment overrides.
NOVYWAVE_WEBKIT_PROFILE=auto novywave

# Older NVIDIA/WebKitGTK workaround.
NOVYWAVE_WEBKIT_PROFILE=legacy-nvidia novywave

# Troubleshooting fallback for driver/compositor issues.
NOVYWAVE_WEBKIT_PROFILE=software novywave
```

## Chrome Launcher

The Chrome launcher is distributed as a one-file AppImage such as `NovyWave_x.x.x_chrome-linux-x64.AppImage`.
Use it when the standard Linux desktop AppImage is slow or blank because of WebKitGTK rendering:

```bash
chmod +x NovyWave_x.x.x_chrome-linux-x64.AppImage
./NovyWave_x.x.x_chrome-linux-x64.AppImage
```

The AppImage includes the backend and frontend assets. It requires Chrome, Chromium, Edge, Brave, Vivaldi, Snap Chromium, or a supported Chromium Flatpak with Chromium 109 or newer.
If browser detection fails, a desktop error dialog or notification should explain what was checked.
If browser detection fails, set one of:

```bash
NOVYWAVE_CHROME=/path/to/google-chrome ./NovyWave_x.x.x_chrome-linux-x64.AppImage
CHROME_BIN=/path/to/chromium ./NovyWave_x.x.x_chrome-linux-x64.AppImage
```

## File Associations

To associate waveform files with NovyWave, create a MIME type file:

```bash
cat > ~/.local/share/mime/packages/novywave.xml << EOF
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-vcd">
    <comment>Value Change Dump</comment>
    <glob pattern="*.vcd"/>
  </mime-type>
  <mime-type type="application/x-fst">
    <comment>Fast Signal Trace</comment>
    <glob pattern="*.fst"/>
  </mime-type>
  <mime-type type="application/x-ghw">
    <comment>GHDL Waveform</comment>
    <glob pattern="*.ghw"/>
  </mime-type>
</mime-info>
EOF

update-mime-database ~/.local/share/mime
```
