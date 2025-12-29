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
    margin-bottom: 20px;
    border-radius: 15px;
  }

  .about-container h2 {
    font-size: 24px;
    font-weight: 600;
    margin: 0 0 8px 0;
  }

  .about-version {
    font-size: 16px;
    color: rgba(255, 255, 255, 0.7);
    margin: 0;
  }

  @media (prefers-color-scheme: light) {
    .about-version {
      color: rgba(0, 0, 0, 0.7);
    }
  }
</style>

