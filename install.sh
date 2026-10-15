#!/bin/bash
# Installs the Scanopy daemon. Run the command your Scanopy server shows on its install page.
# Daemon docs: https://scanopy.net/docs/setting-up-daemons/installing-daemons/
set -e

REPO="scanopy/scanopy"

# --gateway: download through get.scanopy.net (Scarf), falling back to GitHub. Scanopy Cloud and
# commercial servers pass it in the commands they generate. Without it the binary comes straight
# from GitHub. Other arguments are ignored.
USE_GATEWAY=false
for arg in "$@"; do
    case "$arg" in
        --gateway) USE_GATEWAY=true ;;
    esac
done

PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')

case "$PLATFORM" in
    mingw*|msys*|cygwin*)
        echo "Windows detected. This script is for Linux, macOS, and FreeBSD."
        echo ""
        echo "On Windows, install with the signed MSI (attended wizard or silent"
        echo "'msiexec /qn SERVERURL=... APIKEY=...'), or go to the Scanopy web UI and create a"
        echo "daemon for the PowerShell download + 'scanopy-daemon install' commands."
        exit 1
        ;;
esac

ARCH=$(uname -m)

# Map architecture names to match release binaries
case "$ARCH" in
    # FreeBSD reports amd64 where Linux and macOS report x86_64.
    x86_64|amd64)
        ARCH="amd64"
        ;;
    aarch64|arm64)
        ARCH="arm64"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        echo "Supported architectures: x86_64 (amd64), aarch64/arm64"
        exit 1
        ;;
esac

BINARY_NAME="scanopy-daemon-${PLATFORM}-${ARCH}"

echo "Downloading Scanopy daemon..."
echo "Platform: $PLATFORM"
echo "Architecture: $ARCH"
echo "Binary: $BINARY_NAME"
echo ""

# With --gateway, download the latest binary through Scarf, which counts the download and redirects
# to the GitHub release asset, and fall back to GitHub directly when Scarf is down or blocked.
SCARF_URL="https://get.scanopy.net/latest/${BINARY_NAME}"
GITHUB_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}"

DOWNLOADED=false
if [ "$USE_GATEWAY" = true ]; then
    echo "Downloading from: $SCARF_URL"
    if curl -fL --connect-timeout 10 "$SCARF_URL" -o scanopy-daemon; then
        DOWNLOADED=true
    else
        echo "get.scanopy.net unreachable, downloading from GitHub instead"
    fi
fi

if [ "$DOWNLOADED" = false ]; then
    echo "Downloading from: $GITHUB_URL"
    if ! curl -fL "$GITHUB_URL" -o scanopy-daemon; then
        echo "Error: Failed to download binary from $GITHUB_URL"
        echo "Please check:"
        echo "  1. Your internet connection"
        echo "  2. That a release exists for your platform"
        echo "  3. GitHub releases: https://github.com/${REPO}/releases/latest"
        echo "  4. Troubleshooting: https://scanopy.net/docs/setting-up-daemons/troubleshooting-setup/"
        exit 1
    fi
fi

chmod +x scanopy-daemon

# Install to system
echo "Downloading to /usr/local/bin (may require sudo)..."
if [ -w "/usr/local/bin" ]; then
    mv scanopy-daemon /usr/local/bin/
else
    sudo mv scanopy-daemon /usr/local/bin/ || {
        echo "Error: Failed to download scanopy-daemon. Please check sudo permissions."
        rm -f scanopy-daemon
        exit 1
    }
fi

# Verify installation
if [ ! -f "/usr/local/bin/scanopy-daemon" ]; then
    echo "Error: Download verification failed."
    exit 1
fi

echo ""
echo "✓ Scanopy daemon binary downloaded successfully!"
echo ""
