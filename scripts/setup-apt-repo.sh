#!/bin/bash
# Setup script to initialize the gh-pages branch for APT repository

set -e

echo "═══════════════════════════════════════════"
echo "Setting up APT Repository (GitHub Pages)"
echo "═══════════════════════════════════════════"
echo ""

# Check if gh-pages branch exists
if git show-ref --verify --quiet refs/heads/gh-pages; then
    echo "✅ gh-pages branch already exists"
else
    echo "Creating gh-pages branch..."
    
    # Create orphan branch
    git checkout --orphan gh-pages
    
    # Remove all files
    git rm -rf .
    
    # Create initial structure
    mkdir -p pool/main dists/stable/main/binary-amd64
    
    # Create placeholder index
    cat > index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Proxy VM Wizard APT Repository</title>
</head>
<body>
    <h1>Proxy VM Wizard APT Repository</h1>
    <p>This page will be automatically updated when releases are published.</p>
    <p>See <a href="https://github.com/MehdiConnect4/proxy-vm-wizard">GitHub</a> for more information.</p>
</body>
</html>
EOF
    
    # Create .nojekyll to prevent GitHub Pages from processing with Jekyll
    touch .nojekyll
    
    # Commit
    git add .
    git commit -m "Initialize APT repository"
    
    # Push
    git push origin gh-pages
    
    echo "✅ gh-pages branch created and pushed"
    
    # Switch back to main
    git checkout main
fi

echo ""
echo "═══════════════════════════════════════════"
echo "✅ APT Repository Setup Complete!"
echo "═══════════════════════════════════════════"
echo ""
echo "Next steps:"
echo "1. Go to GitHub Settings → Pages"
echo "2. Set Source to 'gh-pages' branch"
echo "3. Wait a few minutes for GitHub Pages to deploy"
echo "4. Your repo will be at: https://mehdiconnect4.github.io/proxy-vm-wizard"
echo ""

