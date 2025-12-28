# Tauri Updater Setup Guide

## Overview
GitHub Releases can automatically push updates to your Tauri app. Here's how to set it up:

## Step 1: Generate Signing Keys

Run the key generation script:
```bash
chmod +x scripts/generate-updater-keys.sh
./scripts/generate-updater-keys.sh
```

Or manually:
```bash
tauri signer generate -w ~/.tauri/fileflow.key
```

**Save the output:**
- **Private Key**: Add to GitHub Secrets as `TAURI_SIGNING_PRIVATE_KEY`
- **Public Key**: Add to `tauri.conf.json` > `bundle` > `updater` > `pubkey`

## Step 2: Update Configuration

1. Edit `src-tauri/tauri.conf.json`:
   - Replace `YOUR_USERNAME` and `YOUR_REPO` in the `endpoints` URL
   - Replace `YOUR_UPDATER_PUBLIC_KEY_HERE` with your public key

2. The endpoint should look like:
   ```json
   "endpoints": [
     "https://github.com/yourusername/folder-watcher/releases/latest/download/latest.json"
   ]
   ```

## Step 3: Add GitHub Secrets

Go to your GitHub repo → Settings → Secrets and variables → Actions:

Add these secrets:
- `TAURI_SIGNING_PRIVATE_KEY`: Your private key from Step 1
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: (optional) Password if you set one

## Step 4: Install Dependencies

```bash
npm install
```

## Step 5: Create a Release

When you push a tag, the workflow will:
1. Build the app
2. Generate update artifacts (`.tar.gz` and `.sig` files)
3. Generate `latest.json` manifest
4. Create a GitHub Release with all files

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Step 6: Check for Updates in Your App

Import and use the update checker:

```javascript
import { checkForUpdates } from './update-checker.js';

// Check on app startup
checkForUpdates(true); // silent check

// Or add a menu item to check manually
// In your menu handler:
checkForUpdates(false); // shows dialog
```

## How It Works

1. **User opens app** → App checks `latest.json` from GitHub Releases
2. **If update available** → Shows dialog (if `dialog: true`)
3. **User confirms** → Downloads `.tar.gz` update file
4. **Verifies signature** → Uses public key to verify update
5. **Installs update** → Extracts and replaces app
6. **Relaunches** → App restarts with new version

## Manual Release (Alternative)

If you prefer manual releases:

1. Build locally: `npm run tauri:build`
2. Upload to GitHub Release:
   - `FileFlow.dmg` (for new installs)
   - `FileFlow.app.tar.gz` (update file)
   - `FileFlow.app.tar.gz.sig` (signature)
   - `latest.json` (manifest - generated in `src-tauri/target/release/bundle/macos/`)

## Troubleshooting

- **Updates not found**: Check the endpoint URL matches your repo
- **Signature verification fails**: Ensure public key in config matches private key used to sign
- **Update artifacts missing**: Make sure `bundle.updater.active` is `true` in config

