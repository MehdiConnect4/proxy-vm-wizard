#!/bin/bash
# Update APT repository to latest GitHub release
set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
    echo "Usage: ./update-apt-repo.sh VERSION"
    echo "Example: ./update-apt-repo.sh 0.2.7"
    exit 1
fi

echo "════════════════════════════════════════════"
echo "Updating APT repository to v${VERSION}"
echo "════════════════════════════════════════════"
echo ""

# Check if release exists
echo "1️⃣  Checking if GitHub release exists..."
if ! curl -sI "https://github.com/MehdiConnect4/proxy-vm-wizard/releases/download/v${VERSION}/proxy-vm-wizard_${VERSION}-1_amd64.deb" | grep -q "HTTP/2 200"; then
    echo "❌ Release v${VERSION} not found or .deb doesn't exist yet"
    echo "Wait for GitHub Actions to finish building"
    echo "Check: https://github.com/MehdiConnect4/proxy-vm-wizard/actions"
    exit 1
fi
echo "✅ Release found!"
echo ""

# Switch to gh-pages
echo "2️⃣  Switching to gh-pages branch..."
git checkout gh-pages
echo ""

# Download new .deb
echo "3️⃣  Downloading .deb from release..."
rm -f pool/main/*.deb
wget -q https://github.com/MehdiConnect4/proxy-vm-wizard/releases/download/v${VERSION}/proxy-vm-wizard_${VERSION}-1_amd64.deb -P pool/main/
echo "✅ Downloaded!"
echo ""

# Regenerate APT metadata
echo "4️⃣  Regenerating APT metadata..."
dpkg-scanpackages --arch amd64 pool/ /dev/null > dists/stable/main/binary-amd64/Packages 2>/dev/null
gzip -9c < dists/stable/main/binary-amd64/Packages > dists/stable/main/binary-amd64/Packages.gz
echo "✅ Packages updated!"
echo ""

# Update Release file
echo "5️⃣  Updating Release file..."
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
echo "✅ Release file updated!"
echo ""

# Commit and push
echo "6️⃣  Committing and pushing..."
git add -f pool/main/*.deb dists/
git commit -m "Update APT repo to v${VERSION}"
git push origin gh-pages
echo "✅ Pushed to gh-pages!"
echo ""

# Switch back to main
git checkout main

echo ""
echo "════════════════════════════════════════════"
echo "✅ DONE! APT repo updated to v${VERSION}"
echo "════════════════════════════════════════════"
echo ""
echo "Wait 2 minutes for GitHub Pages to deploy"
echo ""
echo "Then test:"
echo "  sudo apt update"
echo "  sudo apt upgrade proxy-vm-wizard"
echo ""

