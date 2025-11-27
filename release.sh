#!/bin/bash
# Complete release script - handles EVERYTHING
set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
    echo "Usage: ./release.sh VERSION"
    echo "Example: ./release.sh 0.2.8"
    exit 1
fi

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║              🚀 RELEASING v${VERSION}                                   ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

# Check we're on main
BRANCH=$(git branch --show-current)
if [ "$BRANCH" != "main" ]; then
    echo "❌ Not on main branch! Currently on: $BRANCH"
    echo "Run: git checkout main"
    exit 1
fi

# Check everything is committed
if ! git diff-index --quiet HEAD --; then
    echo "❌ You have uncommitted changes!"
    echo "Commit or stash them first"
    exit 1
fi

echo "1️⃣  Pushing main branch..."
git push || { echo "❌ Failed to push main"; exit 1; }
echo "✅ Main pushed"
echo ""

echo "2️⃣  Creating and pushing tag v${VERSION}..."
# Delete old tag if exists
git push origin :refs/tags/v${VERSION} 2>/dev/null || true
git tag -d v${VERSION} 2>/dev/null || true
git tag v${VERSION}
git push origin v${VERSION} || { echo "❌ Failed to push tag"; exit 1; }
echo "✅ Tag pushed"
echo ""

echo "3️⃣  Waiting for GitHub Actions to build packages..."
echo "    This takes ~10 minutes..."
echo ""

# Wait and check every 30 seconds
MAX_WAIT=600  # 10 minutes
ELAPSED=0
while [ $ELAPSED -lt $MAX_WAIT ]; do
    if curl -sI "https://github.com/MehdiConnect4/proxy-vm-wizard/releases/download/v${VERSION}/proxy-vm-wizard_${VERSION}-1_amd64.deb" | grep -qE "HTTP/2 (200|302)"; then
        echo "✅ Build finished! Packages are ready"
        break
    fi
    sleep 30
    ELAPSED=$((ELAPSED + 30))
    echo "    Still building... ($ELAPSED seconds elapsed)"
done

if [ $ELAPSED -ge $MAX_WAIT ]; then
    echo "⚠️  Build taking longer than expected"
    echo "Check: https://github.com/MehdiConnect4/proxy-vm-wizard/actions"
    echo ""
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo ""
echo "4️⃣  Updating APT repository (gh-pages)..."
echo ""

# Switch to gh-pages
git checkout gh-pages

# Download new .deb
echo "    Downloading .deb..."
rm -f pool/main/*.deb
wget -q https://github.com/MehdiConnect4/proxy-vm-wizard/releases/download/v${VERSION}/proxy-vm-wizard_${VERSION}-1_amd64.deb -P pool/main/

# Regenerate APT metadata
echo "    Regenerating metadata..."
dpkg-scanpackages --arch amd64 pool/ /dev/null > dists/stable/main/binary-amd64/Packages 2>/dev/null
gzip -9c < dists/stable/main/binary-amd64/Packages > dists/stable/main/binary-amd64/Packages.gz

# Update Release file
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

# Add checksums
cd dists/stable
echo "MD5Sum:" >> Release
find main -type f -exec md5sum {} \; | sed 's|main/| |' | awk '{printf " %s %16s %s\n", $1, $3, $2}' >> Release
echo "SHA256:" >> Release
find main -type f -exec sha256sum {} \; | sed 's|main/| |' | awk '{printf " %s %16s %s\n", $1, $3, $2}' >> Release
cd ../..

# Commit and push
echo "    Pushing to gh-pages..."
git add -f pool/main/*.deb dists/
git commit -m "Update APT repo to v${VERSION}"
git push origin gh-pages

# Switch back to main
git checkout main

echo "✅ APT repo updated"
echo ""

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║                      ✅ RELEASE COMPLETE!                           ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""
echo "📦 Released v${VERSION} with:"
echo "  • .deb (Debian/Ubuntu)"
echo "  • .rpm (Fedora/RHEL)" 
echo "  • AppImage"
echo "  • Tarball"
echo ""
echo "🌐 Release page:"
echo "  https://github.com/MehdiConnect4/proxy-vm-wizard/releases/tag/v${VERSION}"
echo ""
echo "📋 APT repo updated (wait 2 mins for GitHub Pages)"
echo "  Users can: sudo apt update && sudo apt upgrade"
echo ""
echo "════════════════════════════════════════════════════════════════════"
echo "🎉 YOUR USERS CAN NOW INSTALL v${VERSION}!"
echo "════════════════════════════════════════════════════════════════════"

