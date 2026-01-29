#!/bin/bash

# check for Cargo.toml
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found in current directory."
    exit 1
fi

# Function to extract current version from Cargo.toml
get_current_version() {
    # Grep the version line, look for the first occurrence (package version)
    # Remove quotes and whitespace
    grep "^version" Cargo.toml | head -n 1 | sed 's/version = "\(.*\)"/\1/'
}

CURRENT_VERSION=$(get_current_version)
IFS='.' read -r -a PARTS <<< "$CURRENT_VERSION"
MAJOR=${PARTS[0]}
MINOR=${PARTS[1]}
PATCH=${PARTS[2]}

echo "------------------------------------------------"
echo "Up Version"
echo "   Current Version: $CURRENT_VERSION"
echo "------------------------------------------------"
echo "Select update type:"
echo "1) Patch ($MAJOR.$MINOR.$((PATCH+1))) - Bug fixes"
echo "2) Minor ($MAJOR.$((MINOR+1)).0) - New features (backwards compatible)"
echo "3) Major ($((MAJOR+1)).0.0) - Breaking changes"
echo "4) Custom Input"
echo "5) Cancel"
echo "------------------------------------------------"

read -p "Select an option [1-5]: " OPTION

case $OPTION in
    1)
        NEW_VERSION="$MAJOR.$MINOR.$((PATCH+1))"
        ;;
    2)
        NEW_VERSION="$MAJOR.$((MINOR+1)).0"
        ;;
    3)
        NEW_VERSION="$((MAJOR+1)).0.0"
        ;;
    4)
        read -p "Enter custom version: " NEW_VERSION
        ;;
    5)
        echo "Aborting."
        exit 0
        ;;
    *)
        echo "Invalid option."
        exit 1
        ;;
esac

echo ""
echo "Targeting Version: $NEW_VERSION"
read -p "Are you sure? (y/n): " CONFIRM

if [[ $CONFIRM != "y" ]]; then
    echo "Cancelled."
    exit 0
fi

# 1. Update Cargo.toml
# We use sed to replace only the first occurrence of version = "..."
# This avoids messing up dependency versions lower down in the file.
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS requires an empty string for the backup extension
    sed -i '' "0,/^version = .*/s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
else
    # Standard Linux sed
    sed -i "0,/^version = .*/s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
fi

echo "✅ Updated Cargo.toml"

# 2. Update Cargo.lock
echo "🔄 Updating Cargo.lock..."
cargo check --quiet > /dev/null 2>&1

# 3. Git Operations
read -p "Do you want to commit and tag v$NEW_VERSION? (y/n): " GIT_CONFIRM

if [[ $GIT_CONFIRM == "y" ]]; then
    git submodule update --remote
    git add Cargo.toml Cargo.lock tricorder stress-ng
    git commit -m "chore: bump version to $NEW_VERSION"
    git tag "v$NEW_VERSION"

    echo "✅ Git commit and tag created."
    echo ""
    echo "Don't forget to push:"
    echo "  git push && git push --tags"
else
    echo "✅ Version updated locally. No git changes made."
fi
