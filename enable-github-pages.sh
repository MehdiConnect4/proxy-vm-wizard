#!/bin/bash
# Enable GitHub Pages via API (no browser needed!)

echo "════════════════════════════════════════════"
echo "Enabling GitHub Pages via CLI..."
echo "════════════════════════════════════════════"

# Get GitHub token from environment or git credentials
GITHUB_TOKEN=$(git config --get github.token 2>/dev/null)

if [ -z "$GITHUB_TOKEN" ]; then
    echo ""
    echo "⚠️  Need GitHub token for API access"
    echo ""
    echo "Get one here: https://github.com/settings/tokens"
    echo "Permissions needed: repo"
    echo ""
    read -p "Paste your GitHub token: " GITHUB_TOKEN
fi

# Enable Pages
RESPONSE=$(curl -s -X POST \
  -H "Authorization: token $GITHUB_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  https://api.github.com/repos/MehdiConnect4/proxy-vm-wizard/pages \
  -d '{"source":{"branch":"gh-pages","path":"/"}}')

if echo "$RESPONSE" | grep -q "html_url"; then
    echo ""
    echo "✅ GitHub Pages enabled!"
    echo "URL: https://mehdiconnect4.github.io/proxy-vm-wizard"
    echo ""
    echo "Wait 2-3 minutes for deployment..."
else
    echo ""
    echo "Response: $RESPONSE"
    echo ""
    echo "If it says 'already enabled' - that's good!"
fi
