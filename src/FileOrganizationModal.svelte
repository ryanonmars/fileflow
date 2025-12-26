<script>
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';

  let pendingFiles = [];
  let processingFile = null;
  let processingAll = false;
  let pollingInterval = null;

  onMount(async () => {
    await loadPendingFiles();
    
    // Poll for updates
    pollingInterval = setInterval(async () => {
      await loadPendingFiles();
    }, 1000);
    
    // Set up refresh function for backend to call
    window.refreshFileList = async () => {
      await loadPendingFiles();
    };
  });

  onDestroy(() => {
    if (pollingInterval) {
      clearInterval(pollingInterval);
    }
  });

  async function loadPendingFiles() {
    try {
      const files = await invoke('get_pending_files');
      // Remove duplicates based on path (in case backend didn't catch them)
      const uniqueFiles = [];
      const seenPaths = new Set();
      for (const file of files) {
        if (!seenPaths.has(file.path)) {
          seenPaths.add(file.path);
          uniqueFiles.push(file);
        }
      }
      pendingFiles = uniqueFiles;
      
      // If no files, close the modal
      if (pendingFiles.length === 0) {
        await closeWindow();
      }
    } catch (err) {
      console.error('Failed to load pending files:', err);
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
    if (processingFile === filePath) return;
    processingFile = filePath;

    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });

      if (selected) {
        const destination = Array.isArray(selected) ? selected[0] : selected;
        await invoke('process_pending_file', {
          filePath: filePath,
          destination: destination,
        });
        await loadPendingFiles();
      }
    } catch (err) {
      console.error('Failed to select destination:', err);
      alert('Error: ' + err);
    } finally {
      processingFile = null;
    }
  }

  async function skipFile(filePath) {
    if (processingFile === filePath) return;
    processingFile = filePath;

    try {
      await invoke('process_pending_file', {
        filePath: filePath,
        destination: null,
      });
      await loadPendingFiles();
    } catch (err) {
      console.error('Failed to skip file:', err);
      alert('Error: ' + err);
    } finally {
      processingFile = null;
    }
  }

  async function skipAll() {
    if (processingAll || pendingFiles.length === 0) return;
    processingAll = true;

    try {
      for (const file of pendingFiles) {
        await invoke('process_pending_file', {
          filePath: file.path,
          destination: null,
        });
      }
      await loadPendingFiles();
    } catch (err) {
      console.error('Failed to skip all files:', err);
      alert('Error: ' + err);
    } finally {
      processingAll = false;
    }
  }

  async function moveAll() {
    if (processingAll || pendingFiles.length === 0) return;
    processingAll = true;

    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });

      if (selected) {
        const destination = Array.isArray(selected) ? selected[0] : selected;
        for (const file of pendingFiles) {
          await invoke('process_pending_file', {
            filePath: file.path,
            destination: destination,
          });
        }
        await loadPendingFiles();
      }
    } catch (err) {
      console.error('Failed to move all files:', err);
      alert('Error: ' + err);
    } finally {
      processingAll = false;
    }
  }

  async function closeWindow() {
    try {
      await invoke('close_file_organization_modal');
    } catch (err) {
      console.error('Failed to close window:', err);
    }
  }
</script>

<div class="modal-content">
  {#if pendingFiles.length > 0}
    <div class="bulk-actions">
      <button class="skip-all-btn" on:click={skipAll} disabled={processingAll}>
        Skip All
      </button>
      <button class="move-all-btn" on:click={moveAll} disabled={processingAll}>
        Move All
      </button>
    </div>
    <div class="file-list">
      {#each pendingFiles as file (file.path + '-' + file.detected_at)}
        <div class="file-item">
          <div class="file-info">
            <span class="file-name">{file.name}</span>
            <span class="file-extension">{file.extension || 'no ext'}</span>
            <span class="file-size">{formatFileSize(file.size)}</span>
          </div>
          <div class="file-actions">
            <button 
              class="select-btn" 
              on:click={() => selectDestination(file.path)}
              disabled={processingFile === file.path || processingAll}
            >
              {#if processingFile === file.path}Processing...{:else}Select{/if}
            </button>
            <button 
              class="skip-btn" 
              on:click={() => skipFile(file.path)}
              disabled={processingFile === file.path || processingAll}
            >
              Skip
            </button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="no-files">
      <p>No files awaiting organization.</p>
      <button class="close-btn" on:click={closeWindow}>Close</button>
    </div>
  {/if}
</div>

<style>
  .modal-content {
    width: 100%;
    height: 100%;
    padding: 20px;
    display: flex;
    flex-direction: column;
    color: rgba(255, 255, 255, 0.9);
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', 'Segoe UI', sans-serif;
    font-size: 13px;
    letter-spacing: -0.01em;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  @media (prefers-color-scheme: light) {
    .modal-content {
      color: rgba(0, 0, 0, 0.9);
    }
  }

  .bulk-actions {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 0.5px solid rgba(255, 255, 255, 0.1);
  }

  @media (prefers-color-scheme: light) {
    .bulk-actions {
      border-bottom-color: rgba(0, 0, 0, 0.1);
    }
  }

  .skip-all-btn,
  .move-all-btn {
    padding: 6px 12px;
    border-radius: 6px;
    border: none;
    font-size: 13px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.15s ease-out;
    letter-spacing: -0.01em;
  }

  .skip-all-btn {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
  }

  @media (prefers-color-scheme: light) {
    .skip-all-btn {
      background: rgba(0, 0, 0, 0.05);
      color: rgba(0, 0, 0, 0.8);
      border: 0.5px solid rgba(0, 0, 0, 0.2);
    }
  }

  .skip-all-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.3);
    transform: translateY(-0.5px);
  }

  @media (prefers-color-scheme: light) {
    .skip-all-btn:hover:not(:disabled) {
      background: rgba(0, 0, 0, 0.1);
      border-color: rgba(0, 0, 0, 0.3);
    }
  }

  .move-all-btn {
    background: #007AFF;
    color: white;
  }

  .move-all-btn:hover:not(:disabled) {
    background: #0051D5;
    transform: translateY(-0.5px);
  }

  .skip-all-btn:active:not(:disabled),
  .move-all-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .skip-all-btn:disabled,
  .move-all-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .file-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    flex: 1;
  }

  .file-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-bottom: 0.5px solid rgba(255, 255, 255, 0.1);
  }

  @media (prefers-color-scheme: light) {
    .file-item {
      border-bottom-color: rgba(0, 0, 0, 0.1);
    }
  }

  .file-item:last-child {
    border-bottom: none;
  }

  .file-info {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .file-name {
    font-weight: 500;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.9);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 300px;
  }

  @media (prefers-color-scheme: light) {
    .file-name {
      color: rgba(0, 0, 0, 0.9);
    }
  }

  .file-extension {
    text-transform: uppercase;
    font-weight: 500;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
    flex-shrink: 0;
  }

  @media (prefers-color-scheme: light) {
    .file-extension {
      color: rgba(0, 0, 0, 0.6);
    }
  }

  .file-size {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
    flex-shrink: 0;
  }

  @media (prefers-color-scheme: light) {
    .file-size {
      color: rgba(0, 0, 0, 0.6);
    }
  }

  .file-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  button {
    padding: 6px 12px;
    border-radius: 6px;
    border: none;
    font-size: 13px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.15s ease-out;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }

  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .select-btn {
    background: #007AFF;
    color: white;
  }

  .select-btn:hover:not(:disabled) {
    background: #0051D5;
    transform: translateY(-0.5px);
  }

  .select-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .skip-btn {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.8);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
  }

  @media (prefers-color-scheme: light) {
    .skip-btn {
      background: rgba(0, 0, 0, 0.05);
      color: rgba(0, 0, 0, 0.8);
      border: 0.5px solid rgba(0, 0, 0, 0.2);
    }
  }

  .skip-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.3);
    transform: translateY(-0.5px);
  }

  @media (prefers-color-scheme: light) {
    .skip-btn:hover:not(:disabled) {
      background: rgba(0, 0, 0, 0.1);
      border-color: rgba(0, 0, 0, 0.3);
    }
  }

  .skip-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.6);
    font-size: 20px;
    cursor: pointer;
    padding: 0 5px;
    line-height: 1;
    transition: color 0.15s ease-out;
  }

  @media (prefers-color-scheme: light) {
    .close-btn {
      color: rgba(0, 0, 0, 0.6);
    }
  }

  .close-btn:hover {
    color: rgba(255, 255, 255, 0.9);
  }

  @media (prefers-color-scheme: light) {
    .close-btn:hover {
      color: rgba(0, 0, 0, 0.9);
    }
  }

  .no-files {
    text-align: center;
    padding: 20px;
    color: rgba(255, 255, 255, 0.6);
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 0.5px dashed rgba(255, 255, 255, 0.2);
    font-size: 13px;
  }

  @media (prefers-color-scheme: light) {
    .no-files {
      color: rgba(0, 0, 0, 0.6);
      background: rgba(0, 0, 0, 0.05);
      border: 0.5px dashed rgba(0, 0, 0, 0.2);
    }
  }

  .no-files p {
    margin-bottom: 1rem;
  }
</style>
