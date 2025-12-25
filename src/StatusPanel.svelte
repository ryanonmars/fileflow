<script>
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  export let onError;
  export let onSuccess;

  let watchedFolder = '';
  let isWatching = false;
  let config = null;
  let organizationMode = 'auto';

  onMount(async () => {
    await loadConfig();
    await loadOrganizationMode();
  });

  async function loadConfig() {
    try {
      config = await invoke('get_config');
      watchedFolder = config.watched_folder || '';
      isWatching = watchedFolder !== '';
    } catch (err) {
      onError(`Failed to load config: ${err}`);
    }
  }

  async function loadOrganizationMode() {
    try {
      organizationMode = await invoke('get_organization_mode');
    } catch (err) {
      onError(`Failed to load organization mode: ${err}`);
    }
  }

  async function changeOrganizationMode() {
    try {
      await invoke('set_organization_mode', { mode: organizationMode });
      onSuccess(`Organization mode set to ${organizationMode}`);
    } catch (err) {
      onError(`Failed to set organization mode: ${err}`);
    }
  }

  async function selectFolder() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
      });
      
      if (selected) {
        watchedFolder = Array.isArray(selected) ? selected[0] : selected;
        if (config) {
          config.watched_folder = watchedFolder;
          await invoke('save_config', { config });
          if (isWatching) {
            await startWatching();
          }
        }
      }
    } catch (err) {
      onError(`Failed to select folder: ${err}`);
    }
  }

  async function startWatching() {
    if (!watchedFolder) {
      onError('Please select a folder to watch');
      return;
    }

    try {
      await invoke('start_watching', { watchedFolder });
      isWatching = true;
      onSuccess('Started watching folder');
    } catch (err) {
      onError(`Failed to start watching: ${err}`);
    }
  }

  async function stopWatching() {
    try {
      await invoke('stop_watching');
      isWatching = false;
      onSuccess('Stopped watching folder');
    } catch (err) {
      onError(`Failed to stop watching: ${err}`);
    }
  }
</script>

<section class="status-panel">
  <h2>Watched Folder</h2>
  
  <p class="background-note">
    The app runs in the background. Close this window to hide it in the menu bar. 
    Files will be processed automatically or you'll receive notifications.
  </p>
  
  <div class="folder-selector">
    <input type="text" bind:value={watchedFolder} placeholder="No folder selected" readonly />
    <button on:click={selectFolder}>Select Folder</button>
  </div>

  <div class="mode-selector">
    <label for="org-mode">Organization Mode:</label>
    <select id="org-mode" bind:value={organizationMode} on:change={changeOrganizationMode}>
      <option value="auto">Auto</option>
      <option value="ask">Ask</option>
      <option value="both">Both</option>
    </select>
  </div>

  <div class="controls">
    <button on:click={startWatching} disabled={isWatching || !watchedFolder}>
      Start Watching
    </button>
    <button on:click={stopWatching} disabled={!isWatching}>
      Stop Watching
    </button>
  </div>

  {#if isWatching}
    <div class="status-indicator">
      <span class="dot active"></span>
      <span>Watching: {watchedFolder}</span>
    </div>
  {/if}
</section>

<style>
  .status-panel {
    background: #2d2d2d;
    padding: 1.5rem;
    border-radius: 8px;
    margin-bottom: 2rem;
    border: 1px solid #444;
  }

  h2 {
    margin-bottom: 1rem;
    font-size: 1.5rem;
    color: #e0e0e0;
  }

  .background-note {
    color: #aaa;
    font-size: 0.9em;
    margin-bottom: 1rem;
    padding: 0.75rem;
    background: #3a3a3a;
    border-radius: 4px;
    border: 1px solid #555;
  }

  .folder-selector {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .folder-selector input {
    flex: 1;
  }

  .mode-selector {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .mode-selector label {
    min-width: 140px;
    color: #e0e0e0;
  }

  .mode-selector select {
    flex: 1;
    max-width: 200px;
  }

  .controls {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: #1a1a1a;
    border-radius: 4px;
    border: 1px solid #444;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #666;
  }

  .dot.active {
    background: #4caf50;
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
</style>
