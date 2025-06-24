#!/bin/bash

# ClearCache Installation Script
# This script builds the project and creates a symbolic link for easy access

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧹 ClearCache Installation Script${NC}"
echo "=================================="

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo (Rust) is not installed. Please install Rust first:${NC}"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Cargo.toml not found. Please run this script from the project root directory.${NC}"
    exit 1
fi

# Build the project in release mode
echo -e "${YELLOW}🔨 Building ClearCache in release mode...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Build failed. Please check the error messages above.${NC}"
    exit 1
fi

# Get the binary path
BINARY_PATH="$(pwd)/target/release/clearcache"

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Binary not found at $BINARY_PATH${NC}"
    exit 1
fi

# Determine the best location for the symlink
SYMLINK_DIRS=(
    "$HOME/.local/bin"
    "$HOME/bin"
    "/usr/local/bin"
)

SYMLINK_DIR=""
for dir in "${SYMLINK_DIRS[@]}"; do
    if [ -d "$dir" ] && [ -w "$dir" ]; then
        SYMLINK_DIR="$dir"
        break
    fi
done

# If no writable directory found, try to create ~/.local/bin
if [ -z "$SYMLINK_DIR" ]; then
    mkdir -p "$HOME/.local/bin"
    if [ $? -eq 0 ]; then
        SYMLINK_DIR="$HOME/.local/bin"
    else
        echo -e "${RED}❌ Could not find or create a suitable directory for the symlink.${NC}"
        echo "Please manually copy the binary from $BINARY_PATH to a directory in your PATH."
        exit 1
    fi
fi

SYMLINK_PATH="$SYMLINK_DIR/clearcache"

# Remove existing symlink if it exists
if [ -L "$SYMLINK_PATH" ]; then
    echo -e "${YELLOW}🔄 Removing existing symlink...${NC}"
    rm "$SYMLINK_PATH"
fi

# Create the symlink
echo -e "${YELLOW}🔗 Creating symlink: $SYMLINK_PATH -> $BINARY_PATH${NC}"
ln -s "$BINARY_PATH" "$SYMLINK_PATH"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Installation successful!${NC}"
    echo ""
    echo -e "${BLUE}Usage:${NC}"
    echo "  clearcache                    # Clean current directory"
    echo "  clearcache /path/to/dir       # Clean specific directory"
    echo "  clearcache --recursive        # Clean recursively"
    echo "  clearcache --dry-run          # Show what would be deleted"
    echo "  clearcache --types node,rust  # Clean specific cache types"
    echo "  clearcache --help             # Show all options"
    echo ""
    
    # Check if the symlink directory is in PATH
    if [[ ":$PATH:" != *":$SYMLINK_DIR:"* ]]; then
        echo -e "${YELLOW}⚠️  Note: $SYMLINK_DIR is not in your PATH.${NC}"
        echo "Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "  export PATH=\"$SYMLINK_DIR:\$PATH\""
        echo ""
    fi
    
    echo -e "${GREEN}🎉 ClearCache is ready to use!${NC}"
else
    echo -e "${RED}❌ Failed to create symlink.${NC}"
    echo "You can manually copy the binary:"
    echo "  cp $BINARY_PATH $SYMLINK_PATH"
    exit 1
fi 