#!/bin/bash
#
# Shakedown Distribution Build Script
#
# This script builds the Shakedown application and prepares it for distribution.
# It assumes stress-ng will be built separately and placed in the dist directory.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DIST_DIR="${PROJECT_DIR}/dist"
CURRENT_TAG=$(git tag -l | tail -n 1)

echo "========================================"
echo "  Shakedown Distribution Builder"
echo "========================================"
echo ""

# Check if we're in the right directory
if [ ! -f "${PROJECT_DIR}/Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Please run this script from the shakedown directory."
    exit 1
fi

# Clean previous dist
if [ -d "$DIST_DIR" ]; then
    echo "Cleaning previous distribution..."
    rm -rf "$DIST_DIR"
fi

# Create dist directory structure
echo "Creating distribution directory..."
mkdir -p "$DIST_DIR"
mkdir -p "$DIST_DIR/jobfiles"

# Build the Rust application in release mode
echo ""
echo "Building Shakedown (release mode)..."
cd "$PROJECT_DIR"
cargo build --release

# Copy the binary
echo "Copying binary..."
cp "${PROJECT_DIR}/target/release/shakedown" "$DIST_DIR/"

# Copy jobfiles
echo "Copying jobfiles..."
cp "${PROJECT_DIR}/jobfiles/"*.job "$DIST_DIR/jobfiles/"

# Clone submodule
git submodule update --init --recursive

# Build Stress-ng
echo "Building Stress-ng..."
cd "$PROJECT_DIR/stress-ng"
make -j$(nproc) STATIC=1
cp stress-ng "$DIST_DIR/"


# Build Tricorder
echo "Building Tricorder..."
cd "$PROJECT_DIR/tricorder"
cargo build --release
cp target/release/tricorder "$DIST_DIR/"


# Create dist tarball with git tag as version
git tag -l | tail -n 1 | xargs tar -czvf "$DIST_DIR/shakedown-$CURRENT_TAG.tar.gz" -C "$DIST_DIR" .

# Print summary
echo ""
echo "========================================"
echo "  Build Complete!"
echo "========================================"
echo ""
