#!/bin/bash

# Configuration
REPO_OWNER="JDKarner"
REPO_NAME="shakedown"
HOST="https://github.com"
TARGET_DIR="shakedown"
BINARY_NAME="shakedown"

# 1. Fetch the latest release tag dynamically
# We use the API to get the JSON for the latest release, then filter for the "tag_name"
echo "Checking for latest version..."
LATEST_TAG=$(curl -sSL "${HOST}/api/v1/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" | grep -oP '"tag_name":\s*"\K[^"]+')

if [ -z "$LATEST_TAG" ]; then
    echo "Error: Could not determine latest version. Check your internet connection or repository URL."
    exit 1
fi

echo "Latest version identified: $LATEST_TAG"

# Construct the dynamic URL
# Assuming the file format follows your pattern: shakedown-[tag].tar.gz
FILE_NAME="shakedown-${LATEST_TAG}.tar.gz"
DOWNLOAD_URL="${HOST}/${REPO_OWNER}/${REPO_NAME}/releases/download/${LATEST_TAG}/${FILE_NAME}"

# 2. Create the directory & Download
mkdir -p "$TARGET_DIR"

echo "Downloading $FILE_NAME..."
curl -L "$DOWNLOAD_URL" -o "$TARGET_DIR/$FILE_NAME"

if [ $? -ne 0 ]; then
    echo "Error: Download failed. Verify the asset exists for release $LATEST_TAG"
    exit 1
fi

# 3. Extract
echo "Extracting..."
tar -xf "$TARGET_DIR/$FILE_NAME" -C "$TARGET_DIR"

# 4. Set Permissions (Simplified) && cd
# It's safer/faster to just chmod the binary you know you need
chmod +x "$TARGET_DIR/$BINARY_NAME"
cd "$TARGET_DIR"
sudo apt update
sudo apt install smartmontools

# 5. Run
echo "Running $BINARY_NAME..."
"./$BINARY_NAME"
