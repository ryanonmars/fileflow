<script>
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';

  let pendingFiles = [];
  let processingFile = null;
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

  async function closeWindow() {
    try {
      await invoke('close_file_organization_modal');
    } catch (err) {
      console.error('Failed to close window:', err);
    }
  }
</script>

<div class="modal-container">
  <div class="modal-content">
    <div class="header">
      <h2>Files Awaiting Organization</h2>
      <button class="close-btn" on:click={closeWindow}>×</button>
    </div>
    
    {#if pendingFiles.length > 0}
      <div class="file-list">
        {#each pendingFiles as file (file.path + '-' + file.detected_at)}
          <div class="file-item">
            <div class="file-info">
              <div class="file-name">{file.name}</div>
              <div class="file-details">
                <span class="file-extension">{file.extension || 'no extension'}</span>
                <span class="file-size">{formatFileSize(file.size)}</span>
                <span class="file-time">Detected: {formatTimestamp(file.detected_at)}</span>
              </div>
              <div class="file-path">{file.path}</div>
            </div>
            <div class="file-actions">
              <button 
                class="select-btn" 
                on:click={() => selectDestination(file.path)}
                disabled={processingFile === file.path}
              >
                {#if processingFile === file.path}Processing...{:else}Select Destination{/if}
              </button>
              <button 
                class="skip-btn" 
                on:click={() => skipFile(file.path)}
                disabled={processingFile === file.path}
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
</div>

<style>
  .modal-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #1a1a1a;
    color: #e0e0e0;
    padding: 2rem;
  }

  .modal-content {
    background: #2d2d2d;
    border-radius: 12px;
    padding: 2rem;
    max-width: 700px;
    width: 100%;
    max-height: 80vh;
    border: 1px solid #444;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  h2 {
    margin: 0;
    font-size: 1.5rem;
    color: #e0e0e0;
  }

  .file-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
    flex: 1;
  }

  .file-item {
    background: #3a3a3a;
    border-radius: 8px;
    padding: 1rem;
    border: 1px solid #555;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .file-info {
    flex: 1;
    min-width: 0;
  }

  .file-name {
    font-weight: 600;
    font-size: 1.1rem;
    color: #4a90e2;
    margin-bottom: 0.5rem;
    word-break: break-all;
  }

  .file-details {
    display: flex;
    gap: 1rem;
    font-size: 0.9em;
    color: #aaa;
    margin-bottom: 0.5rem;
    flex-wrap: wrap;
  }

  .file-extension {
    font-weight: 500;
  }

  .file-size {
    font-weight: 500;
  }

  .file-time {
    color: #888;
  }

  .file-path {
    font-size: 0.85em;
    word-break: break-all;
    color: #888;
    margin-top: 0.5rem;
  }

  .file-actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  button {
    padding: 0.75em 1.5em;
    border-radius: 6px;
    border: none;
    font-size: 0.95em;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
    white-space: nowrap;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .select-btn {
    background: #4a90e2;
    color: white;
  }

  .select-btn:hover:not(:disabled) {
    background: #357abd;
  }

  .skip-btn {
    background: #3a3a3a;
    color: #e0e0e0;
    border: 1px solid #555;
  }

  .skip-btn:hover:not(:disabled) {
    background: #4a4a4a;
  }

  .close-btn {
    background: transparent;
    color: #aaa;
    border: 1px solid #555;
    padding: 0.5em 1em;
    font-size: 1.2em;
    line-height: 1;
  }

  .close-btn:hover:not(:disabled) {
    background: #3a3a3a;
    color: #e0e0e0;
  }

  .no-files {
    text-align: center;
    padding: 2rem;
    color: #aaa;
  }

  .no-files p {
    margin-bottom: 1rem;
  }
</style>
