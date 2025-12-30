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
  <h2 class="monitored-folder-title">Monitored Folder</h2>
  
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
    background: #252525;
    padding: 0.75rem 1rem;
    padding-bottom: 0.5rem;
    border-radius: 4px;
    margin-bottom: 0.75rem;
    border: 1px solid #333;
  }

  h2 {
    margin: 0 0 0.75rem 0;
    font-size: 1.1rem;
    font-weight: 500;
    color: #e0e0e0;
  }

  .monitored-folder-title {
    font-size: 0.9rem;
  }

  .folder-selector {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .folder-selector input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    font-size: 0.9em;
  }

  .mode-selector {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }

  .mode-selector label {
    font-size: 0.9em;
    color: #bbb;
    min-width: 120px;
  }

  .mode-selector select {
    flex: 0 0 auto;
    padding: 0.5rem 0.75rem;
    font-size: 0.9em;
    min-width: 120px;
  }

  .controls {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  button {
    padding: 0.5rem 1rem;
    font-size: 0.9em;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: #1f1f1f;
    border-radius: 3px;
    border: 1px solid #333;
    font-size: 0.9em;
    margin-top: 0.25rem;
    margin-bottom: 0;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #666;
    flex-shrink: 0;
  }

  .dot.active {
    background: #4caf50;
  }

  .status-indicator span:last-child {
    color: #bbb;
    font-size: 0.9em;
  }
</style>
