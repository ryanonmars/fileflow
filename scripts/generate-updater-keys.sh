#!/bin/bash
# Generate Tauri updater signing keys
# Run this once and save the keys securely

echo "Generating Tauri updater signing keys..."
echo ""

# Generate keypair
tauri signer generate -w ~/.tauri/myapp.key

echo ""
echo "========================================="
echo "IMPORTANT: Save these keys securely!"
echo "========================================="
echo ""
echo "Private Key (save as GitHub Secret: TAURI_SIGNING_PRIVATE_KEY):"
cat ~/.tauri/myapp.key
echo ""
echo ""
echo "Public Key (add to tauri.conf.json > bundle > updater > pubkey):"
tauri signer generate -w ~/.tauri/myapp.key --print-public-key
echo ""
echo "========================================="


