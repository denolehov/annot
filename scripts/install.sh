#!/bin/bash
set -e

# annot installer for macOS
# Requires: gh CLI with repo access

REPO="denolehov/annot"
APP_DIR="$HOME/.local/share/annot"
BIN_DIR="$HOME/.local/bin"

# Detect platform
case "$(uname -s)" in
    Darwin) ;;
    *) echo "Error: Only macOS is supported" >&2; exit 1 ;;
esac

case "$(uname -m)" in
    arm64|aarch64) platform="darwin-arm64" ;;
    x86_64|amd64) platform="darwin-x64" ;;
    *) echo "Error: Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

echo "Installing annot for $platform..."

# Check for gh CLI
if ! command -v gh &> /dev/null; then
    echo "Error: GitHub CLI (gh) is required."
    echo ""
    echo "Install it with:"
    echo "  brew install gh"
    echo ""
    echo "Then authenticate:"
    echo "  gh auth login"
    exit 1
fi

# Check gh auth
if ! gh auth status &> /dev/null 2>&1; then
    echo "Error: Not authenticated with GitHub."
    echo ""
    echo "Run:"
    echo "  gh auth login"
    exit 1
fi

# Check repo access
if ! gh repo view "$REPO" &> /dev/null 2>&1; then
    echo "Error: No access to $REPO"
    echo "Make sure your GitHub account has access to this private repo."
    exit 1
fi

# Create temp directory
TEMP_DIR=$(mktemp -d)
cleanup() { rm -rf "$TEMP_DIR"; }
trap cleanup EXIT
cd "$TEMP_DIR"

echo "Downloading latest release..."
gh release download --repo "$REPO" --pattern "*.tar.gz"

ARCHIVE="annot-${platform}.tar.gz"
if [[ ! -f "$ARCHIVE" ]]; then
    # Fallback for current naming
    ARCHIVE="annot-darwin-arm64.tar.gz"
fi

if [[ ! -f "$ARCHIVE" ]]; then
    echo "Error: No compatible release found for $platform" >&2
    exit 1
fi

echo "Extracting..."
# Clear quarantine on archive before extraction
xattr -d com.apple.quarantine "$ARCHIVE" 2>/dev/null || true
tar -xzf "$ARCHIVE"

# Verify app exists
if [[ ! -d "annot.app" ]]; then
    echo "Error: annot.app not found in archive" >&2
    exit 1
fi

# Clear all extended attributes immediately after extraction (before SIP locks it)
xattr -cr annot.app 2>/dev/null || true

# Verify signature
echo "Verifying code signature..."
if ! codesign -v annot.app 2>/dev/null; then
    echo "Warning: Code signature verification failed"
fi

# Create directories
mkdir -p "$APP_DIR"
mkdir -p "$BIN_DIR"

# Remove old installation
rm -rf "$APP_DIR/annot.app"
rm -f "$BIN_DIR/annot"

# Install .app bundle (required for code signing to work)
echo "Installing to $APP_DIR/annot.app..."
mv annot.app "$APP_DIR/"

# Create symlink for CLI usage
ln -sf "$APP_DIR/annot.app/Contents/MacOS/annot" "$BIN_DIR/annot"

# Verify installation
if "$BIN_DIR/annot" --version &>/dev/null; then
    echo "Verified: annot is working"
else
    echo "Warning: Installation may have issues. Try running: $BIN_DIR/annot --help"
fi

# Check if bin dir is in PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo ""
    echo "Add to your shell config (~/.zshrc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo ""
echo "Done! Run 'annot --help' to get started."
