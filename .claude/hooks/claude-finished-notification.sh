#!/bin/bash
# Claude Code Stop Notification

source "$(dirname "$0")/shared-functions.sh"
init_hook_env

# Desktop notification
notify-send "🤖 Claude finished!" --urgency=critical --icon=face-robot --expire-time=8000 --category=im.received

# Sound notification (try multiple methods for compatibility)
# Method 1: paplay (PulseAudio)
if command -v paplay >/dev/null 2>&1; then
    paplay /usr/share/sounds/sound-icons/glass-water-1.wav 2>/dev/null &
fi

# Method 2: aplay (ALSA)
if command -v aplay >/dev/null 2>&1; then
    aplay /usr/share/sounds/sound-icons/glass-water-1.wav 2>/dev/null &
fi

# Method 3: espeak (text-to-speech as fallback)
if command -v espeak >/dev/null 2>&1; then
    espeak "Claude finished" 2>/dev/null &
fi

# Method 4: System bell as last resort
printf "\a"

# Clean up session markers for next session
rm -f "$PROJECT_ROOT/.claude/session-recovery-completed" 2>/dev/null || true

# Log session end for debugging
echo "🏁 Session ended, markers cleaned: $(date)" >> "$HOOK_LOG"