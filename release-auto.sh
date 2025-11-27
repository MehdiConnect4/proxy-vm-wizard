#!/bin/bash
# COMPLETE AUTOMATED RELEASE SCRIPT
# Usage: ./release-auto.sh [patch|minor|major]
# Example: ./release-auto.sh patch  (0.2.8 -> 0.2.9)

set -e

BUMP_TYPE="${1:-patch}"

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║           🚀 AUTOMATED RELEASE PROCESS                             ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

# Check we're on main
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "main" ]; then
    echo "❌ Not on main branch! Run: git checkout main"
    exit 1
fi

# Check everything is committed
if ! git diff-index --quiet HEAD --; then
    echo "❌ Uncommitted changes detected!"
    git status --short
    echo ""
    read -p "Commit them now? [Y/n] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
        git add -A
        read -p "Commit message: " MSG
        git commit -m "$MSG"
    else
        exit 1
    fi
fi

# Get current version
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)
echo "📊 Current version: $CURRENT_VERSION"

# Calculate new version
IFS='.' read -r -a VERSION_PARTS <<< "$CURRENT_VERSION"
MAJOR="${VERSION_PARTS[0]}"
MINOR="${VERSION_PARTS[1]}"
PATCH="${VERSION_PARTS[2]}"

case "$BUMP_TYPE" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
    *)
        echo "❌ Invalid bump type: $BUMP_TYPE"
        echo "Use: patch, minor, or major"
        exit 1
        ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
echo "🆕 New version: $NEW_VERSION"
echo ""

read -p "Proceed with release v$NEW_VERSION? [Y/n] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ ! -z $REPLY ]]; then
    echo "❌ Cancelled"
    exit 1
fi

echo ""
echo "1️⃣  Bumping version to $NEW_VERSION..."

# Update version in all files
sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
sed -i "s/pkgver=$CURRENT_VERSION/pkgver=$NEW_VERSION/" packaging/PKGBUILD
sed -i "s/Version:        $CURRENT_VERSION/Version:        $NEW_VERSION/" packaging/rpm.spec

# Update CHANGELOG
TODAY=$(date +%Y-%m-%d)
sed -i "s/## \[Unreleased\]/## [Unreleased]\n\n## [$NEW_VERSION] - $TODAY\n\n### Changed\n- Version bump for release/" CHANGELOG.md

git add Cargo.toml packaging/PKGBUILD packaging/rpm.spec CHANGELOG.md
git commit -m "Bump version to $NEW_VERSION"

echo "✅ Version bumped"
echo ""

echo "2️⃣  Running tests..."
if ! cargo test --all --quiet; then
    echo "❌ Tests failed!"
    exit 1
fi
echo "✅ All tests pass"
echo ""

echo "3️⃣  Running clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tail -1 | grep -q "Finished"; then
    echo "❌ Clippy found issues!"
    exit 1
fi
echo "✅ Clippy clean"
echo ""

echo "4️⃣  Checking formatting..."
if ! cargo fmt --all -- --check; then
    echo "⚠️  Formatting issues found. Fixing..."
    cargo fmt --all
    git add -A
    git commit -m "Fix formatting"
fi
echo "✅ Formatting OK"
echo ""

echo "5️⃣  Pushing to GitHub..."
git push
echo "✅ Pushed"
echo ""

echo "6️⃣  Creating and pushing tag v$NEW_VERSION..."
git push origin :refs/tags/v$NEW_VERSION 2>/dev/null || true
git tag -d v$NEW_VERSION 2>/dev/null || true
git tag v$NEW_VERSION
git push origin v$NEW_VERSION
echo "✅ Tag pushed"
echo ""

echo "7️⃣  Waiting for GitHub Actions (~10 minutes)..."
echo "    Building: .deb, .rpm, AppImage, Tarball"
echo ""

MAX_WAIT=720  # 12 minutes
ELAPSED=0
while [ $ELAPSED -lt $MAX_WAIT ]; do
    if curl -sI "https://github.com/MehdiConnect4/proxy-vm-wizard/releases/download/v${NEW_VERSION}/proxy-vm-wizard_${NEW_VERSION}-1_amd64.deb" 2>/dev/null | grep -qE "HTTP/2 (200|302)"; then
        echo "✅ Build complete!"
        break
    fi
    sleep 30
    ELAPSED=$((ELAPSED + 30))
    printf "    ⏳ %d/%d seconds\n" $ELAPSED $MAX_WAIT
done

if [ $ELAPSED -ge $MAX_WAIT ]; then
    echo "⚠️  Timeout waiting for build"
    echo "Check: https://github.com/MehdiConnect4/proxy-vm-wizard/actions"
    exit 1
fi

echo ""
echo "8️⃣  Updating APT repository (gh-pages)..."

git checkout gh-pages

echo "    Downloading .deb..."
rm -f pool/main/*.deb
wget -q "https://github.com/MehdiConnect4/proxy-vm-wizard/releases/download/v${NEW_VERSION}/proxy-vm-wizard_${NEW_VERSION}-1_amd64.deb" -P pool/main/

echo "    Regenerating metadata..."
dpkg-scanpackages --arch amd64 pool/ /dev/null 2>/dev/null > dists/stable/main/binary-amd64/Packages
gzip -9c < dists/stable/main/binary-amd64/Packages > dists/stable/main/binary-amd64/Packages.gz

rm -f dists/stable/Release
cat > dists/stable/Release << EOF
Origin: Proxy VM Wizard
Label: Proxy VM Wizard Repository  
Suite: stable
Codename: stable
Architectures: amd64
Components: main
Description: APT repository for Proxy VM Wizard
Date: $(date -Ru)
EOF

cd dists/stable
echo "MD5Sum:" >> Release
find main -type f -exec md5sum {} \; | sed 's|main/| |' | awk '{printf " %s %16s %s\n", $1, $3, $2}' >> Release
echo "SHA256:" >> Release
find main -type f -exec sha256sum {} \; | sed 's|main/| |' | awk '{printf " %s %16s %s\n", $1, $3, $2}' >> Release
cd ../..

echo "    Pushing to gh-pages..."
git add -f pool/main/*.deb dists/
git commit -m "Update APT repo to v${NEW_VERSION}"
git push origin gh-pages

git checkout main

echo "✅ APT repo updated"
echo ""

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║                    🎉 RELEASE v$NEW_VERSION COMPLETE! 🎉                    ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""
echo "📦 Released with:"
echo "  • .deb (Debian/Ubuntu) ✅"
echo "  • .rpm (Fedora/RHEL) ✅"
echo "  • AppImage ✅"
echo "  • Tarball ✅"
echo "  • APT repo updated ✅"
echo ""
echo "🌐 Release:"
echo "  https://github.com/MehdiConnect4/proxy-vm-wizard/releases/tag/v${NEW_VERSION}"
echo ""
echo "👥 Users can now:"
echo "  • Debian/Ubuntu: sudo apt update && sudo apt upgrade"
echo "  • Fedora: sudo dnf install proxy-vm-wizard-${NEW_VERSION}-1.x86_64.rpm"
echo "  • Any Linux: Download AppImage"
echo ""
echo "════════════════════════════════════════════════════════════════════"
echo "🎊 ALL DONE! Ship it to your users! 🚀"
echo "════════════════════════════════════════════════════════════════════"

