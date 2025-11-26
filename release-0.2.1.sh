#!/bin/bash
# Release v0.2.1 - Performance fix

set -e

echo "🚀 Releasing v0.2.1..."
echo ""

echo "1️⃣  Pushing main branch..."
git push

echo ""
echo "2️⃣  Pushing gh-pages (APT repo base)..."
git checkout gh-pages
git push origin gh-pages
git checkout main

echo ""
echo "3️⃣  Creating and pushing tag..."
git tag v0.2.1
git push origin v0.2.1

echo ""
echo "════════════════════════════════════════════"
echo "✅ DONE!"
echo "════════════════════════════════════════════"
echo ""
echo "GitHub Actions is now building:"
echo "  • .deb package"
echo "  • AppImage"
echo "  • Tarball"
echo ""
echo "Wait 10 minutes, then check:"
echo "  https://github.com/MehdiConnect4/proxy-vm-wizard/releases"
echo ""
echo "APT repo will auto-update with new .deb"
echo "Users can: sudo apt update && apt upgrade"
echo ""
