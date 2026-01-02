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

    // Modal window is now shown automatically by the backend
    // No need to listen for file-queued events here
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
          filePath: filePath, 
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
        filePath: filePath, 
        destination: null 
      });
      await loadPendingFiles();
      onSuccess('File skipped');
    } catch (err) {
      onError(`Failed to skip file: ${err}`);
    }
  }

  async function deleteFile(filePath) {
    const { ask } = await import('@tauri-apps/plugin-dialog');
    const confirmed = await ask('Are you sure you want to delete this file? This action cannot be undone.', {
      title: 'Delete File',
      kind: 'warning'
    });
    
    if (!confirmed) {
      return;
    }
    
    try {
      await invoke('delete_pending_file', { filePath: filePath });
      await loadPendingFiles();
      onSuccess('File deleted');
    } catch (err) {
      onError(`Failed to delete file: ${err}`);
    }
  }
</script>

<section class="pending-panel">
  <h2>Pending Files</h2>
  
  <p class="notification-note">
    Files queued in "Ask" or "Both" mode will also trigger system notifications. 
    You can process them here or via the notifications.
  </p>
  
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
            <button class="delete-btn" on:click={() => deleteFile(file.path)}>
              Delete
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .pending-panel {
    background: #252525;
    padding: 0.75rem 1rem;
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

  .notification-note {
    color: #999;
    font-size: 0.85em;
    margin-bottom: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: #1f1f1f;
    border-radius: 3px;
    border: 1px solid #333;
  }

  .empty-state {
    text-align: center;
    padding: 1.5rem;
    color: #999;
    background: #1f1f1f;
    border-radius: 3px;
    border: 1px dashed #333;
    font-size: 0.9em;
  }

  .files-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .file-item {
    background: #1f1f1f;
    border-radius: 3px;
    padding: 0.75rem;
    border: 1px solid #333;
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
    margin-bottom: 0.25rem;
    word-break: break-all;
    font-size: 0.9em;
  }

  .file-details {
    display: flex;
    gap: 0.75rem;
    font-size: 0.85em;
    color: #999;
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
    padding: 0.4rem 0.75rem;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.85em;
  }

  .select-btn:hover {
    background: #357abd;
  }

  .skip-btn {
    background: transparent;
    color: #bbb;
    border: 1px solid #444;
    padding: 0.4rem 0.75rem;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.85em;
  }

  .skip-btn:hover {
    background: #2a2a2a;
    border-color: #555;
  }

  .delete-btn {
    background: transparent;
    color: #ff453a;
    border: 1px solid #ff453a;
    padding: 0.4rem 0.75rem;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.85em;
  }

  .delete-btn:hover {
    background: #ff453a;
    color: white;
  }
</style>

