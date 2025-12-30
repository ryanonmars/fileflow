<script>
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import appIcon from './app-icon.png';

  let version = '';
  let productName = 'FileFlow';

  onMount(async () => {
    const appWindow = getCurrentWindow();
    
    // Listen for close event - hide instead of close so window can be reopened
    appWindow.listen('tauri://close-requested', async (event) => {
      event.preventDefault();
      await appWindow.hide();
    });

    // Get version and product name from backend
    try {
      const [appVersion, appName] = await invoke('get_app_info');
      version = appVersion;
      productName = appName;
    } catch (err) {
      console.error('Failed to get app info:', err);
      version = '0.1.1';
      productName = 'FileFlow';
    }
  });
</script>

<div class="about-container">
  <img src={appIcon} alt="FileFlow" class="about-logo" />
  <h2>{productName}</h2>
  <p class="about-version">Version {version}</p>
  <button class="about-link" on:click={async () => {
    try {
      const timestamp = Date.now();
      await invoke('open_url', { url: `https://htmlpreview.github.io/?https://raw.githubusercontent.com/ryanonmars/fileflow/main/RELEASE_NOTES.html?t=${timestamp}` });
    } catch (err) {
      console.error('Failed to open URL:', err);
    }
  }}>
    Release Notes
  </button>
  <button class="about-link" on:click={async () => {
    try {
      await invoke('open_url', { url: 'https://github.com/ryanonmars/fileflow' });
    } catch (err) {
      console.error('Failed to open URL:', err);
    }
  }}>
    GitHub Repository
  </button>
</div>

<style>
  .about-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    padding: 32px;
    background: rgba(30, 30, 30, 0.95);
    color: rgba(255, 255, 255, 0.9);
  }

  @media (prefers-color-scheme: light) {
    .about-container {
      background: rgba(255, 255, 255, 0.95);
      color: rgba(0, 0, 0, 0.9);
    }
  }

  .about-logo {
    width: 80px;
    height: 80px;
    margin-bottom: 0;
    border-radius: 15px;
  }

  .about-container h2 {
    font-size: 24px;
    font-weight: 600;
    margin: -4px 0 8px 0;
  }

  .about-version {
    font-size: 16px;
    color: rgba(255, 255, 255, 0.7);
    margin: 0 0 16px 0;
  }

  @media (prefers-color-scheme: light) {
    .about-version {
      color: rgba(0, 0, 0, 0.7);
    }
  }

  .about-link {
    font-size: 14px;
    color: #007AFF;
    text-decoration: none;
    margin-top: 8px;
    transition: opacity 0.2s ease;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: inherit;
  }

  .about-link:hover {
    opacity: 0.8;
    text-decoration: underline;
  }

  @media (prefers-color-scheme: light) {
    .about-link {
      color: #007AFF;
    }
  }
</style>

