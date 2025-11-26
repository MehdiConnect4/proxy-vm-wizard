# APT Repository Setup

This document explains how the APT repository works for Proxy VM Wizard.

## For Users

Add the repository and install:

```bash
# Add repository
echo "deb [trusted=yes] https://mehdiconnect4.github.io/proxy-vm-wizard stable main" | sudo tee /etc/apt/sources.list.d/proxy-vm-wizard.list

# Update and install
sudo apt update
sudo apt install proxy-vm-wizard
```

Updates will be automatic:
```bash
sudo apt update && sudo apt upgrade
```

## For Maintainers

### Initial Setup (One-Time)

1. Run the setup script:
   ```bash
   ./scripts/setup-apt-repo.sh
   ```

2. Enable GitHub Pages:
   - Go to: https://github.com/MehdiConnect4/proxy-vm-wizard/settings/pages
   - Source: Deploy from branch → `gh-pages` → `/root`
   - Click Save

3. Wait 2-3 minutes for GitHub Pages to deploy

4. Verify it works:
   ```bash
   curl -I https://mehdiconnect4.github.io/proxy-vm-wizard
   ```

### How It Works

1. When you create a GitHub Release with a .deb file, the workflow automatically:
   - Downloads the .deb from the release
   - Adds it to the APT repository structure
   - Generates Packages files and Release metadata
   - Commits to gh-pages branch
   - GitHub Pages hosts the files

2. Users can then:
   - Add the repository once
   - Get automatic updates via `apt update && apt upgrade`

### Repository Structure

```
gh-pages branch:
├── index.html              # Repository homepage
├── pool/main/              # .deb packages stored here
│   └── proxy-vm-wizard_*.deb
├── dists/stable/
│   ├── Release             # Repository metadata
│   └── main/binary-amd64/
│       ├── Packages        # Package index
│       └── Packages.gz     # Compressed package index
└── .nojekyll               # Prevent Jekyll processing
```

### Manual Repository Update

If needed, you can manually trigger the repository update:
- Go to Actions → "Update APT Repository" → Run workflow

### Security Note

This setup uses `[trusted=yes]` to skip GPG signing for simplicity. For production with many users, consider adding GPG signing:

1. Generate GPG key
2. Sign Release files
3. Users import public key
4. Remove `[trusted=yes]`

## Troubleshooting

### Repository not updating
- Check GitHub Actions logs
- Ensure gh-pages branch exists
- Verify GitHub Pages is enabled

### Users can't find package
- Wait 5 minutes after release
- Check repository URL is accessible
- Verify .deb was uploaded to release

### Permission errors
- Ensure GitHub Actions has write permissions
- Check repository settings → Actions → Workflow permissions

