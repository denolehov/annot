#!/bin/bash
set -e

# annot installer for macOS
# Requires: gh CLI with repo access (run `gh auth login` first)

REPO="denolehov/annot"
INSTALL_DIR="$HOME/.local/bin"
APP_NAME="annot"

echo "Installing annot..."

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
if ! gh auth status &> /dev/null; then
    echo "Error: Not authenticated with GitHub."
    echo ""
    echo "Run:"
    echo "  gh auth login"
    exit 1
fi

# Check repo access
if ! gh repo view "$REPO" &> /dev/null; then
    echo "Error: No access to $REPO"
    echo "Make sure your GitHub account has access to this private repo."
    exit 1
fi

# Create temp directory
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo "Downloading latest release..."
gh release download --repo "$REPO" --pattern "*.tar.gz"

echo "Extracting..."
tar -xzf annot-darwin-arm64.tar.gz

# Create install directory if needed
mkdir -p "$INSTALL_DIR"

# Install the binary
echo "Installing to $INSTALL_DIR..."
cp "$APP_NAME.app/Contents/MacOS/$APP_NAME" "$INSTALL_DIR/$APP_NAME"
chmod +x "$INSTALL_DIR/$APP_NAME"

# Cleanup
rm -rf "$TEMP_DIR"

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "Add this to your shell config (~/.zshrc or ~/.bashrc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then restart your terminal or run:"
    echo "  source ~/.zshrc"
fi

echo ""
echo "Done! Run 'annot --help' to get started."
