<script>
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';

  export let onError;
  export let onSuccess;

  let pendingFiles = [];
  let pollingInterval = null;

  onMount(async () => {
    await loadPendingFiles();
    pollingInterval = setInterval(async () => {
      await loadPendingFiles();
    }, 2000);
  });

  onDestroy(() => {
    if (pollingInterval) {
      clearInterval(pollingInterval);
    }
  });

  async function loadPendingFiles() {
    try {
      pendingFiles = await invoke('get_pending_files');
    } catch (err) {
      onError(`Failed to load pending files: ${err}`);
    }
  }

  function formatFileSize(bytes) {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  }

  function formatTimestamp(timestamp) {
    const date = new Date(parseInt(timestamp) * 1000);
    return date.toLocaleString();
  }

  async function selectDestination(filePath) {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
      });
      
      if (selected) {
        const destination = Array.isArray(selected) ? selected[0] : selected;
        await invoke('process_pending_file', { 
          filePath, 
          destination: destination 
        });
        await loadPendingFiles();
        onSuccess('File moved successfully');
      }
    } catch (err) {
      onError(`Failed to process file: ${err}`);
    }
  }

  async function skipFile(filePath) {
    try {
      await invoke('process_pending_file', { 
        filePath, 
        destination: null 
      });
      await loadPendingFiles();
      onSuccess('File skipped');
    } catch (err) {
      onError(`Failed to skip file: ${err}`);
    }
  }
</script>

<section class="pending-panel">
  <h2>Pending Files</h2>
  
  {#if pendingFiles.length === 0}
    <div class="empty-state">
      <p>No pending files. Files will appear here when they need your attention.</p>
    </div>
  {:else}
    <div class="files-list">
      {#each pendingFiles as file (file.path)}
        <div class="file-item">
          <div class="file-info">
            <div class="file-name">{file.name}</div>
            <div class="file-details">
              <span class="file-extension">{file.extension || 'no extension'}</span>
              <span class="file-size">{formatFileSize(file.size)}</span>
              <span class="file-time">{formatTimestamp(file.detected_at)}</span>
            </div>
          </div>
          <div class="file-actions">
            <button class="select-btn" on:click={() => selectDestination(file.path)}>
              Select Destination
            </button>
            <button class="skip-btn" on:click={() => skipFile(file.path)}>
              Skip
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .pending-panel {
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

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: #aaa;
    background: #3a3a3a;
    border-radius: 8px;
    border: 1px dashed #555;
  }

  .files-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .file-item {
    background: #3a3a3a;
    border-radius: 8px;
    padding: 1rem;
    border: 1px solid #555;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .file-info {
    flex: 1;
    min-width: 0;
  }

  .file-name {
    font-weight: 500;
    color: #e0e0e0;
    margin-bottom: 0.5rem;
    word-break: break-all;
  }

  .file-details {
    display: flex;
    gap: 1rem;
    font-size: 0.9em;
    color: #aaa;
    flex-wrap: wrap;
  }

  .file-extension {
    text-transform: uppercase;
    font-weight: 500;
  }

  .file-actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .select-btn {
    background: #4a90e2;
    color: white;
    border: none;
    padding: 0.5em 1em;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9em;
  }

  .select-btn:hover {
    background: #357abd;
  }

  .skip-btn {
    background: #3a3a3a;
    color: #e0e0e0;
    border: 1px solid #555;
    padding: 0.5em 1em;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9em;
  }

  .skip-btn:hover {
    background: #4a4a4a;
  }
</style>

