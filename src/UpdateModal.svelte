<script>
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import appIcon from './app-icon.png';

  let status = 'idle'; // 'idle', 'checking', 'available', 'latest', 'error'
  let updateVersion = '';
  let errorMessage = '';
  let isInstalling = false;
  let autoCheckEnabled = false;

  onMount(async () => {
    const appWindow = getCurrentWindow();
    
    // Listen for close event - hide instead of close so window can be reopened
    appWindow.listen('tauri://close-requested', async (event) => {
      event.preventDefault();
      await appWindow.hide();
    });

    // Listen for update status events
    const unlistenAvailable = await listen('update-available', (event) => {
      status = 'available';
      updateVersion = event.payload.version || '';
    });

    const unlistenLatest = await listen('update-latest', () => {
      status = 'latest';
    });

    const unlistenError = await listen('update-error', (event) => {
      status = 'error';
      errorMessage = event.payload.message || 'Unknown error';
    });

    const unlistenInstalling = await listen('update-installing', () => {
      isInstalling = true;
    });

    // Get auto-check setting and check if enabled
    try {
      const config = await invoke('get_config');
      autoCheckEnabled = config?.auto_check_for_updates !== false;
      
      // Only auto-check if enabled
      if (autoCheckEnabled) {
        setTimeout(async () => {
          try {
            status = 'checking';
            await invoke('check_for_updates');
          } catch (err) {
            console.error('Failed to check for updates:', err);
            status = 'error';
            errorMessage = String(err);
          }
        }, 100);
      }
    } catch (err) {
      console.error('Failed to get config:', err);
      // Default to not auto-checking if we can't get config
    }

    return async () => {
      await unlistenAvailable();
      await unlistenLatest();
      await unlistenError();
      await unlistenInstalling();
    };
  });

  async function installUpdate() {
    try {
      await invoke('install_update');
    } catch (err) {
      console.error('Failed to install update:', err);
      status = 'error';
      errorMessage = String(err);
    }
  }

  async function closeWindow() {
    const appWindow = getCurrentWindow();
    await appWindow.hide();
  }

  async function checkForUpdates() {
    status = 'checking';
    errorMessage = '';
    try {
      await invoke('check_for_updates');
    } catch (err) {
      console.error('Failed to check for updates:', err);
      status = 'error';
      errorMessage = String(err);
    }
  }

  async function ignoreFor7Days() {
    try {
      await invoke('suppress_update_alert_for_days', { days: 7 });
      await closeWindow();
    } catch (err) {
      console.error('Failed to suppress update alert:', err);
    }
  }
</script>

<div class="update-container">
  <img src={appIcon} alt="FileFlow" class="update-logo" />
  
  {#if status === 'idle'}
    <h2>Check for Updates</h2>
    <p class="update-message">Click the button below to check for available updates.</p>
    <button class="update-btn primary" on:click={checkForUpdates}>Check for Updates</button>
  {:else if status === 'checking'}
    <h2>Checking for Updates...</h2>
    <p class="update-message">Please wait while we check for available updates.</p>
  {:else if status === 'available'}
    <h2>Update Available</h2>
    <p class="update-message">Version {updateVersion} is available.</p>
    <p class="update-submessage">Would you like to install it now?</p>
    <div class="update-buttons">
      <button class="update-btn primary" on:click={installUpdate} disabled={isInstalling}>
        {#if isInstalling}
          Installing...
        {:else}
          Update
        {/if}
      </button>
      <button class="update-btn" on:click={closeWindow} disabled={isInstalling}>
        Later
      </button>
      <button class="update-btn" on:click={ignoreFor7Days} disabled={isInstalling}>
        Ignore for 7 days
      </button>
    </div>
  {:else if status === 'latest'}
    <h2>No Updates</h2>
    <p class="update-message">You are on the latest version.</p>
    <button class="update-btn primary" on:click={checkForUpdates}>Check Again</button>
  {:else if status === 'error'}
    <h2>Update Check Failed</h2>
    <p class="update-message error">{errorMessage}</p>
    <button class="update-btn primary" on:click={checkForUpdates}>Try Again</button>
  {/if}
</div>

<style>
  .update-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    min-height: 100vh;
    padding: 24px;
    padding-top: 32px;
    background: rgba(30, 30, 30, 0.95);
    color: rgba(255, 255, 255, 0.9);
    box-sizing: border-box;
  }

  @media (prefers-color-scheme: light) {
    .update-container {
      background: rgba(255, 255, 255, 0.95);
      color: rgba(0, 0, 0, 0.9);
    }
  }

  .update-logo {
    width: 64px;
    height: 64px;
    margin-bottom: 16px;
    border-radius: 12px;
  }

  .update-container h2 {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 8px 0;
  }

  .update-message {
    font-size: 14px;
    color: rgba(255, 255, 255, 0.7);
    margin: 0 0 8px 0;
    text-align: center;
  }

  .update-message.error {
    color: #ff6b6b;
  }

  @media (prefers-color-scheme: light) {
    .update-message {
      color: rgba(0, 0, 0, 0.7);
    }
    .update-message.error {
      color: #d32f2f;
    }
  }

  .update-submessage {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
    margin: 0 0 16px 0;
    text-align: center;
  }

  @media (prefers-color-scheme: light) {
    .update-submessage {
      color: rgba(0, 0, 0, 0.6);
    }
  }

  .update-buttons {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
    width: 100%;
    max-width: 280px;
  }

  .update-btn {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    border: 1px solid rgba(255, 255, 255, 0.2);
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.2s ease;
    width: 100%;
  }

  .update-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.15);
  }

  .update-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .update-btn.primary {
    background: #007AFF;
    color: white;
    border: none;
  }

  .update-btn.primary:hover:not(:disabled) {
    background: #0056b3;
  }

  @media (prefers-color-scheme: light) {
    .update-btn {
      background: rgba(0, 0, 0, 0.05);
      color: rgba(0, 0, 0, 0.9);
      border: 1px solid rgba(0, 0, 0, 0.1);
    }

    .update-btn:hover:not(:disabled) {
      background: rgba(0, 0, 0, 0.1);
    }

    .update-btn.primary {
      background: #007AFF;
      color: white;
    }
  }
</style>

