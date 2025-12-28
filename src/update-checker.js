// Example: Check for updates on app startup or via menu
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export async function checkForUpdates(silent = false) {
  try {
    const update = await check();
    
    if (update?.available) {
      if (silent) {
        // Silent check - just return if update is available
        return { available: true, version: update.version };
      }
      
      // Show update dialog (Tauri handles this if dialog: true in config)
      const shouldUpdate = confirm(
        `Update available: ${update.version}\n\n` +
        `Current version: ${update.currentVersion}\n\n` +
        `Would you like to download and install it now?`
      );
      
      if (shouldUpdate) {
        await update.downloadAndInstall();
        await relaunch();
      }
      
      return { available: true, version: update.version };
    }
    
    if (!silent) {
      alert('You are running the latest version!');
    }
    
    return { available: false };
  } catch (error) {
    console.error('Update check failed:', error);
    if (!silent) {
      alert('Failed to check for updates. Please try again later.');
    }
    return { available: false, error: error.message };
  }
}

