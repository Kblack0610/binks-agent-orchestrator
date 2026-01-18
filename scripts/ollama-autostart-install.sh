#!/bin/bash
#
# Install Ollama auto-start on macOS (Mac Studio)
# This creates a launchd agent that starts Ollama on login
#
# Usage: ssh 192.168.1.4 'bash -s' < scripts/ollama-autostart-install.sh
#    or: ./scripts/ollama-autostart-install.sh (run on Mac Studio directly)

set -e

PLIST_NAME="com.ollama.serve.plist"
LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
PLIST_PATH="$LAUNCH_AGENTS_DIR/$PLIST_NAME"

echo "=== Ollama Auto-Start Installer ==="
echo ""

# Create LaunchAgents directory if needed
mkdir -p "$LAUNCH_AGENTS_DIR"

# Create the plist
cat > "$PLIST_PATH" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.ollama.serve</string>

    <key>ProgramArguments</key>
    <array>
        <string>/opt/homebrew/bin/ollama</string>
        <string>serve</string>
    </array>

    <key>EnvironmentVariables</key>
    <dict>
        <key>OLLAMA_HOST</key>
        <string>0.0.0.0</string>
        <key>OLLAMA_FLASH_ATTENTION</key>
        <string>1</string>
        <key>OLLAMA_CONTEXT_LENGTH</key>
        <string>131072</string>
        <key>OLLAMA_KEEP_ALIVE</key>
        <string>24h</string>
        <key>OLLAMA_MAX_LOADED_MODELS</key>
        <string>2</string>
        <key>PATH</key>
        <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin</string>
    </dict>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>/tmp/ollama.log</string>

    <key>StandardErrorPath</key>
    <string>/tmp/ollama.error.log</string>
</dict>
</plist>
EOF

echo "Created: $PLIST_PATH"

# Unload if already loaded
launchctl unload "$PLIST_PATH" 2>/dev/null || true

# Load the agent
launchctl load "$PLIST_PATH"

echo ""
echo "=== Installation Complete ==="
echo ""
echo "Ollama will now:"
echo "  - Start automatically on login"
echo "  - Restart if it crashes"
echo "  - Listen on 0.0.0.0:11434 (accessible from network)"
echo ""
echo "Beast Mode Settings:"
echo "  - OLLAMA_FLASH_ATTENTION=1"
echo "  - OLLAMA_CONTEXT_LENGTH=131072 (128k)"
echo "  - OLLAMA_KEEP_ALIVE=24h"
echo "  - OLLAMA_MAX_LOADED_MODELS=2"
echo ""
echo "Logs: /tmp/ollama.log, /tmp/ollama.error.log"
echo ""
echo "Commands:"
echo "  launchctl list | grep ollama    # Check status"
echo "  launchctl stop com.ollama.serve # Stop"
echo "  launchctl start com.ollama.serve # Start"
echo "  launchctl unload $PLIST_PATH    # Disable auto-start"
echo ""

# Verify it's running
sleep 2
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✓ Ollama is running!"
else
    echo "⚠ Ollama may still be starting... check logs"
fi
