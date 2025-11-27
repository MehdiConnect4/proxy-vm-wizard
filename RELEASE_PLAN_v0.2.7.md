# Release Plan for v0.2.7

## ✅ AUTOMATIC (GitHub Actions) - NO ACTION NEEDED

GitHub Actions automatically builds:
- ✅ `.deb` package (Debian/Ubuntu)
- ✅ `AppImage` (Universal Linux)
- ✅ `Tarball` (Generic Linux)

**Status**: Check https://github.com/MehdiConnect4/proxy-vm-wizard/actions

**ETA**: 10 minutes

**Result**: All 3 files will be in https://github.com/MehdiConnect4/proxy-vm-wizard/releases/tag/v0.2.7

---

## 📝 MANUAL ACTIONS NEEDED

### 1. Update APT Repository (For Debian/Ubuntu auto-updates)

**After GitHub Actions finishes (10 mins):**

```bash
cd /home/test/Documents/RustApp

# Switch to gh-pages branch
git checkout gh-pages

# Download new .deb from release
rm -f pool/main/*.deb
wget https://github.com/MehdiConnect4/proxy-vm-wizard/releases/download/v0.2.7/proxy-vm-wizard_0.2.7-1_amd64.deb -P pool/main/

# Regenerate APT metadata
dpkg-scanpackages --arch amd64 pool/ /dev/null > dists/stable/main/binary-amd64/Packages
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
git add -f pool/main/*.deb dists/
git commit -m "Update APT repo to v0.2.7"
git push origin gh-pages

# Switch back to main
git checkout main
```

**Result**: Users can `sudo apt update && sudo apt upgrade` to get v0.2.7

---

### 2. Build Fedora RPM (Optional - for Fedora users)

**On a Fedora machine or VM:**

```bash
# Install build tools
sudo dnf install rpm-build rpmdevtools rust cargo

# Build RPM
make rpm

# Upload to GitHub Release
# Go to: https://github.com/MehdiConnect4/proxy-vm-wizard/releases/tag/v0.2.7
# Click "Edit release"
# Upload: ~/rpmbuild/RPMS/x86_64/proxy-vm-wizard-0.2.7-1.x86_64.rpm
```

**OR**: Let Fedora users build from source (instructions in README)

---

### 3. Submit to Flathub (For Flatpak on Flathub)

**Prerequisites:**
```bash
# Install tools
pip3 install aiohttp toml

# Generate cargo sources
python3 flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json
git add flatpak/cargo-sources.json
git commit -m "Add cargo-sources.json for Flathub"
git push
```

**Submission:**
1. Fork https://github.com/flathub/flathub
2. Create new repository: `flathub/io.github.proxyvmwizard.ProxyVmWizard`
3. Copy these files to the new repo:
   - `flatpak/io.github.proxyvmwizard.ProxyVmWizard.yml`
   - `flatpak/flathub.json`
   - `flatpak/cargo-sources.json`
4. Create PR to flathub/flathub adding your app
5. Respond to review feedback
6. Once approved: Users can `flatpak install flathub io.github.proxyvmwizard.ProxyVmWizard`

**See `FLATHUB_SUBMISSION.md` for complete details!**

---

## 🎯 PRIORITY ORDER

### High Priority (Do Now):
1. ✅ Wait for GitHub Actions to finish (10 mins)
2. ✅ Update APT repository (5 mins)
3. ✅ Test `apt upgrade` works

### Medium Priority (This Week):
1. Generate cargo-sources.json
2. Submit to Flathub
3. Wait for Flathub approval (1-2 weeks)

### Low Priority (Optional):
1. Build RPM on Fedora machine
2. Upload RPM to release

---

## ✅ WHAT'S ALREADY DONE

- ✅ Code is ready
- ✅ All tests pass
- ✅ Security hardened
- ✅ Fedora compatible
- ✅ Flathub compliant
- ✅ Documentation complete
- ✅ v0.2.7 tagged and pushed

---

## 📊 VERIFICATION STEPS

### After 10 minutes:

**1. Check GitHub Release exists:**
```bash
curl -s https://api.github.com/repos/MehdiConnect4/proxy-vm-wizard/releases/latest | grep tag_name
# Should show: "tag_name": "v0.2.7"
```

**2. Check .deb is in release:**
```bash
curl -s https://api.github.com/repos/MehdiConnect4/proxy-vm-wizard/releases/latest | grep "proxy-vm-wizard.*deb"
# Should show download URL
```

**3. Update APT repo (see step 1 above)**

**4. Test installation:**
```bash
sudo apt update
sudo apt upgrade proxy-vm-wizard
# Should upgrade to 0.2.7
```

---

## 🎊 DONE!

After these steps:
- ✅ Debian/Ubuntu users: `apt install proxy-vm-wizard`
- ✅ Generic Linux users: Download AppImage or tarball
- ✅ Fedora users: Build from source or use tarball
- ✅ Flatpak users (after Flathub approval): `flatpak install proxy-vm-wizard`

Your app is ready for the world! 🚀

