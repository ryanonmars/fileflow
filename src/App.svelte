<script>
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import FileOrganizationModal from './FileOrganizationModal.svelte';

  let error = '';
  let success = '';
  let isModalWindow = false;
  let activeTab = 'general';

  // General tab state
  let watchedFolder = '';
  let isWatching = false;
  let config = null;
  let organizationMode = 'auto';

  // Rules tab state
  const conditionTypes = [
    { value: 'filetype', label: 'File Type' },
    { value: 'name', label: 'Name' },
    { value: 'created_date', label: 'Created Date' }
  ];
  const dateOperators = [
    { value: 'before', label: 'Before' },
    { value: 'after', label: 'After' },
    { value: 'on', label: 'On' }
  ];
  const commonFileTypes = ['jpg', 'jpeg', 'png', 'gif', 'txt', 'webp', 'svg', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'zip', 'mp4', 'mp3', '*'];
  let rules = [];
  let collapsedRules = {};
  let editingName = null;

  // Pending tab state
  let pendingFiles = [];
  let pollingInterval = null;

  onMount(async () => {
    const appWindow = getCurrentWindow();
    const label = appWindow.label;
    
    if (label === 'file-organization') {
      isModalWindow = true;
    } else {
      appWindow.listen('tauri://close-requested', async () => {
        await appWindow.hide();
      });
      await loadGeneralConfig();
      await loadRules();
      await loadPendingFiles();
      pollingInterval = setInterval(async () => {
        await loadPendingFiles();
      }, 2000);
    }
  });

  function handleError(msg) {
    error = msg;
    success = '';
    setTimeout(() => { error = ''; }, 5000);
  }

  function handleSuccess(msg) {
    success = msg;
    error = '';
    setTimeout(() => { success = ''; }, 3000);
  }

  // General tab functions
  async function loadGeneralConfig() {
    try {
      config = await invoke('get_config');
      watchedFolder = config.watched_folder || '';
      organizationMode = await invoke('get_organization_mode');
      
      // Automatically start watching if a folder is configured
      if (watchedFolder) {
        try {
          await invoke('start_watching', { watchedFolder });
          isWatching = true;
        } catch (err) {
          // If watching fails, don't set isWatching to true
          console.error('Failed to auto-start watching:', err);
          isWatching = false;
        }
      } else {
        isWatching = false;
      }
    } catch (err) {
      handleError(`Failed to load config: ${err}`);
    }
  }

  async function changeOrganizationMode() {
    try {
      await invoke('set_organization_mode', { mode: organizationMode });
      handleSuccess(`Organization mode set to ${organizationMode}`);
    } catch (err) {
      handleError(`Failed to set organization mode: ${err}`);
    }
  }

  let modeDescription = '';

  $: {
    if (organizationMode === 'auto') {
      modeDescription = 'Files are automatically moved to their destination folders based on your rules. No prompts or notifications.';
    } else if (organizationMode === 'ask') {
      modeDescription = 'Every file triggers a prompt asking where to move it. You can select a destination folder or skip the file.';
    } else if (organizationMode === 'both') {
      modeDescription = 'Files matching your rules are automatically moved. Files that don\'t match any rule trigger a prompt asking where to move them.';
    } else {
      modeDescription = '';
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
          await startWatching();
        }
      }
    } catch (err) {
      handleError(`Failed to select folder: ${err}`);
    }
  }

  async function startWatching() {
    if (!watchedFolder) {
      handleError('Please select a folder to watch');
      return;
    }
    try {
      await invoke('start_watching', { watchedFolder });
      isWatching = true;
      handleSuccess('Started watching folder');
    } catch (err) {
      handleError(`Failed to start watching: ${err}`);
    }
  }

  async function clearFolder() {
    try {
      await invoke('stop_watching');
      watchedFolder = '';
      isWatching = false;
      if (config) {
        config.watched_folder = '';
        await invoke('save_config', { config });
      }
      handleSuccess('Cleared watched folder');
    } catch (err) {
      handleError(`Failed to clear folder: ${err}`);
    }
  }

  // Rules tab functions
  function getConditionType(condition) {
    if (condition.type === 'filetype') return 'filetype';
    if (condition.type === 'name') return 'name';
    if (condition.type === 'created_date') return 'created_date';
    return 'filetype';
  }

  function getConditionValue(condition) {
    if (condition.type === 'filetype') return condition.value || '';
    if (condition.type === 'name') return condition.pattern || '';
    if (condition.type === 'created_date') return condition.value || '';
    return '';
  }

  function getOperator(condition) {
    if (condition.type === 'created_date') return condition.operator || 'before';
    return 'before';
  }

  async function loadRules() {
    try {
      const loadedConfig = await invoke('get_config');
      if (!loadedConfig.rules) {
        loadedConfig.rules = [];
      }
      rules = loadedConfig.rules.map((rule, index) => ({
        id: Date.now() + index,
        name: rule.name || `Rule ${index + 1}`,
        conditionType: getConditionType(rule.condition),
        conditionValue: getConditionValue(rule.condition),
        operator: getOperator(rule.condition),
        destination: rule.destination
      }));
      // Default all rules to collapsed
      collapsedRules = {};
      rules.forEach(rule => {
        collapsedRules[rule.id] = true;
      });
    } catch (err) {
      handleError(`Failed to load rules: ${err}`);
    }
  }

  function toggleCollapse(ruleId) {
    collapsedRules[ruleId] = !collapsedRules[ruleId];
    collapsedRules = collapsedRules;
  }

  function addRule() {
    const newId = Date.now();
    rules = [...rules, {
      id: newId,
      name: `Rule ${rules.length + 1}`,
      conditionType: 'filetype',
      conditionValue: '',
      operator: 'before',
      destination: ''
    }];
    collapsedRules[newId] = true;
    collapsedRules = collapsedRules;
    saveRules();
  }

  function removeRule(index) {
    const ruleId = rules[index].id;
    rules = rules.filter((_, i) => i !== index);
    const newCollapsed = { ...collapsedRules };
    delete newCollapsed[ruleId];
    collapsedRules = newCollapsed;
    saveRules();
  }

  async function saveRules() {
    try {
      const loadedConfig = await invoke('get_config');
      loadedConfig.rules = rules.map(rule => {
        let condition;
        if (rule.conditionType === 'filetype') {
          condition = { type: 'filetype', value: rule.conditionValue };
        } else if (rule.conditionType === 'name') {
          condition = { type: 'name', pattern: rule.conditionValue };
        } else if (rule.conditionType === 'created_date') {
          condition = { type: 'created_date', operator: rule.operator, value: rule.conditionValue };
        }
        return {
          name: rule.name,
          condition,
          destination: rule.destination
        };
      });
      await invoke('save_config', { config: loadedConfig });
    } catch (err) {
      handleError(`Failed to save rules: ${err}`);
    }
  }

  async function selectDestinationFolder(ruleIndex) {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
      });
      
      if (selected) {
        const destination = Array.isArray(selected) ? selected[0] : selected;
        rules[ruleIndex].destination = destination;
        rules = rules;
        saveRules();
      }
    } catch (err) {
      handleError(`Failed to select destination folder: ${err}`);
    }
  }

  // Pending tab functions
  async function loadPendingFiles() {
    try {
      const files = await invoke('get_pending_files');
      const uniqueFiles = [];
      const seenPaths = new Set();
      for (const file of files) {
        if (!seenPaths.has(file.path)) {
          seenPaths.add(file.path);
          uniqueFiles.push(file);
        }
      }
      pendingFiles = uniqueFiles;
    } catch (err) {
      handleError(`Failed to load pending files: ${err}`);
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
        handleSuccess('File moved successfully');
      }
    } catch (err) {
      handleError(`Failed to process file: ${err}`);
    }
  }

  async function skipFile(filePath) {
    try {
      await invoke('process_pending_file', { 
        filePath: filePath, 
        destination: null 
      });
      await loadPendingFiles();
      handleSuccess('File skipped');
    } catch (err) {
      handleError(`Failed to skip file: ${err}`);
    }
  }
</script>

{#if isModalWindow}
  <FileOrganizationModal />
{:else}
  <div class="app-container">
    {#if error}
      <div class="message error">{error}</div>
    {/if}
    
    {#if success}
      <div class="message success">{success}</div>
    {/if}

    <div class="tabs">
      <div class="tabs-left">
        <button 
          class="tab" 
          class:active={activeTab === 'general'}
          on:click={() => activeTab = 'general'}
        >
          General
        </button>
        <button 
          class="tab" 
          class:active={activeTab === 'rules'}
          on:click={() => activeTab = 'rules'}
        >
          Rules
        </button>
        <button 
          class="tab" 
          class:active={activeTab === 'pending'}
          on:click={() => activeTab = 'pending'}
        >
          Pending
        </button>
      </div>
      <div class="logo-container">
        <svg class="header-logo" viewBox="0 0 1038.6 775.62" xmlns="http://www.w3.org/2000/svg">
          <path class="logo-path" d="M43.32,443.75v.18c0,2.8,4.04,3.22,4.61.48l50.16-239.33c6.25-29.81,35.92-44.96,64.22-44.95l813.93.11c16.02,0,29.35,2.32,42.32,11.39,13.69,9.58,23.05,26.76,19.14,44.88l-38.14,176.45-69.19,314.34c-7.92,36-34.7,67.61-75.27,67.65l-765.15.68c-14.4.01-26.68-.76-39.1-4.67C17.63,760.5-.03,732.26,0,698.15L.52,63.53c.02-24.03,16.57-44.13,36-54.42C51.09,1.39,65.22.11,81.66.1L340.86,0c27.71-.01,49.56,10.72,67.8,31.09l45.53,50.85c5.74,6.41,11.54,10.66,21.1,10.67l463.92.37c1.1,0,2.17.07,3.2.21,23.64,3.14,20.65,38.58-3.19,38.59l-465.47.13c-14.33,0-28.55-1.18-38.89-11.31-21.76-21.33-41.69-43.91-61.84-66.76-7.56-8.57-15.09-13.76-27.28-13.75l-279.98.13c-10.05,0-22.28,9.09-22.28,19.63l-.15,383.92ZM853.91,736.17c.1,0,.21,0,.31-.02,21.8-2.49,32.21-15.61,36.37-34.39l33.91-153,73.38-330.86c1.58-7.12,2.36-12.76-1.8-18.64-2.46-3.48-8.81-7.5-15.25-7.5H159.67c-.1.01-.2.02-.31.03-15.54,1.68-25.66,12.66-26.44,27.74,0,.09-.02.19-.03.28l-21.35,107.98-71.21,370.28c-2.97,15.45,4.76,29.09,19.02,34.34,9.14,3.37,18.23,3.8,29.01,3.8l765.56-.04Z"/>
          <path class="logo-path" d="M436.34,606.69c-60.3-61.3-58.13-145.6-.33-207,65.97-70.08,216.61-100.49,289.16-21.48-7.17-24.53-25.61-42.46-47.33-54.01-48.28-25.68-113.86-25.39-165.96-11.94-71.42,18.44-139.21,55.51-183.83,115.24-23.27,31.15-37.33,67.19-37.72,106.35-.27,27.47,5.54,54.47,22.31,76.92-9.09-32.9-10.18-62.71-1.4-94.59,16.16-58.73,55.34-108.54,106.16-136.74-44.45,52.08-60.51,121.78-34.94,183.7s81.69,99.68,141.26,122.49c24.63,9.43,48.85,14.27,75.46,21.02-54.84,9.78-108.01,11.82-162.82,12.06-73.02.31-151.99-4.41-219.12-33.89-45.93-20.17-88-57.88-84.74-112.22,2.98-49.67,36.81-93.62,76.85-124.28-30.87,57.04-39.15,122.35,12.05,175.03-1.22-7.33-4.65-12.58-6.64-19.08-19.62-64.13-2.98-126.15,34.83-179.97,27.68-39.39,67.74-65.06,104.63-94.93l29.4-23.8c19.54-15.82,132.16-80.25,162.43-65.39-6.92,5.76-14.66,7.93-22.13,11.65-20.99,10.43-42.34,20.11-61.16,36.23,76.82-31.53,179.27-63.76,259.94-41.65,15.78,4.33,55.07,17.95,45.9,40.85l-22.4-10.46c-39.86-18.61-85.36-21.58-127.84-12.85,69.03.85,139.68,21.62,180.3,79.99,40.77,58.59,12.18,114.91-3.79,107.78-7.95-3.55,13.54-28.14-2.97-66.86-9.18-21.52-24.18-40.69-43.06-53.83,15.82,19.35,27.39,38.04,31.39,61.46,5.26,30.81-4.4,60.97-26.48,83.18-21.98,22.11-52.2,32.7-84.3,28.99,13.1-7.03,24.33-14.7,31.53-27.27,11.37-19.85,6.7-43.95-11.98-57.74-18.03-13.31-40.43-16.18-62.49-12.57-38.17,6.26-66.86,35.46-70.87,74.04-5.9,56.81,42.95,105.2,91.26,128.28,67.49,32.25,144.73,34.34,215.07,8.99,1.46-3.04,1.44-2.21,1.35.13-11.67,12.32-23.77,22.83-38.22,31.3-67.68,39.65-154.55,42.81-227.3,15.74-43.98-16.36-80.81-46.45-104.55-86.31-21.24-35.66-25.37-74.5-14.52-115.68-30.63,42.49-29.32,99.55-2.23,142.7,13.41,21.36,30.67,38.03,51.48,53.09-33.9-9.18-66.05-26.7-91.63-52.69Z"/>
        </svg>
      </div>
    </div>

    <div class="content">

      {#if activeTab === 'general'}
        <div class="tab-content">
          <h2>Watched Folder</h2>
          
          <div class="form-group">
            <label for="watched-folder">Folder Path:</label>
            <div class="folder-selector">
              <input id="watched-folder" type="text" bind:value={watchedFolder} placeholder="No folder selected" readonly />
              <button on:click={selectFolder}>Select Folder</button>
            </div>
          </div>

          <div class="form-group">
            <button on:click={clearFolder} disabled={!watchedFolder}>
              Clear Folder
            </button>
          </div>

          {#if isWatching}
            <div class="status-indicator">
              <span class="dot active"></span>
              <span>Watching: {watchedFolder}</span>
            </div>
          {/if}
        </div>
      {:else if activeTab === 'rules'}
        <div class="tab-content">
          <div class="rules-header">
            <h2>Organization Rules</h2>
            <button class="add-rule-btn" on:click={addRule}>+ Add Rule</button>
          </div>
          
          <div class="form-group org-mode-group">
            <label for="org-mode">Organization Mode:</label>
            <select id="org-mode" bind:value={organizationMode} on:change={changeOrganizationMode}>
              <option value="auto">Auto</option>
              <option value="ask">Ask</option>
              <option value="both">Both</option>
            </select>
            <span class="mode-description">{modeDescription}</span>
          </div>
          
          <div class="rules-list">
            {#each rules as rule, index (rule.id)}
              <div class="rule-item">
                <div 
                  class="rule-header" 
                  role="button"
                  tabindex="0"
                  on:click={() => toggleCollapse(rule.id)}
                  on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { toggleCollapse(rule.id); } }}
                >
                  <div class="rule-header-left">
                    <span class="collapse-btn" class:collapsed={collapsedRules[rule.id]}>▶</span>
                    <input 
                      type="text" 
                      class="rule-name"
                      bind:value={rule.name}
                      on:blur={saveRules}
                      on:keydown={(e) => { if (e.key === 'Enter') e.target.blur(); }}
                      stopPropagation
                    />
                  </div>
                  <button class="remove-btn" on:click|stopPropagation={() => removeRule(index)}>Remove</button>
                </div>
                {#if !collapsedRules[rule.id]}
                  <div class="rule-content">
                    <div class="condition-group">
                      <label for="condition-type-{rule.id}">If:</label>
                      <select id="condition-type-{rule.id}" bind:value={rule.conditionType} on:change={saveRules}>
                        {#each conditionTypes as type}
                          <option value={type.value}>{type.label}</option>
                        {/each}
                      </select>
                    </div>

                    {#if rule.conditionType === 'created_date'}
                      <div class="condition-group">
                        <label for="operator-{rule.id}">Operator:</label>
                        <select id="operator-{rule.id}" bind:value={rule.operator} on:change={saveRules}>
                          {#each dateOperators as op}
                            <option value={op.value}>{op.label}</option>
                          {/each}
                        </select>
                      </div>
                    {/if}

                    <div class="condition-group">
                      <label for="condition-value-{rule.id}">Value:</label>
                      {#if rule.conditionType === 'filetype'}
                        <select id="condition-value-{rule.id}" bind:value={rule.conditionValue} on:change={saveRules}>
                          <option value="">Select file type...</option>
                          {#each commonFileTypes as fileType}
                            <option value={fileType}>{fileType}</option>
                          {/each}
                        </select>
                      {:else if rule.conditionType === 'name'}
                        <input 
                          id="condition-value-{rule.id}"
                          type="text" 
                          bind:value={rule.conditionValue}
                          placeholder="e.g., invoice*, report*.pdf"
                          on:input={saveRules}
                        />
                      {:else if rule.conditionType === 'created_date'}
                        <input 
                          id="condition-value-{rule.id}"
                          type="date" 
                          bind:value={rule.conditionValue}
                          on:change={saveRules}
                        />
                      {/if}
                    </div>

                    <div class="condition-group">
                      <label for="destination-{rule.id}">Then move to:</label>
                      <div class="folder-selector">
                        <input 
                          id="destination-{rule.id}"
                          type="text" 
                          value={rule.destination}
                          placeholder="No folder selected"
                          readonly
                        />
                        <button on:click={() => selectDestinationFolder(index)}>Select Folder</button>
                      </div>
                    </div>
                  </div>
                {/if}
              </div>
            {:else}
              <div class="no-rules">
                <p>No rules configured. Add a rule to start organizing files.</p>
              </div>
            {/each}
          </div>
        </div>
      {:else if activeTab === 'pending'}
        <div class="tab-content">
          <h2>Pending Files</h2>
          
          {#if pendingFiles.length === 0}
            <div class="empty-state">
              <p>No pending files. Files will appear here when they need your attention.</p>
            </div>
          {:else}
            <div class="files-list">
              {#each pendingFiles as file (file.path + '-' + file.detected_at)}
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
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #f5f5f7;
    color: rgba(0, 0, 0, 0.85);
    overflow: hidden;
  }

  @media (prefers-color-scheme: dark) {
    .app-container {
      background: #1c1c1e;
      color: rgba(255, 255, 255, 0.85);
    }
  }

  .tabs {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #ffffff;
    border-bottom: 0.5px solid rgba(0, 0, 0, 0.1);
    padding: 0;
    flex-shrink: 0;
  }

  .tabs-left {
    display: flex;
  }

  .logo-container {
    display: flex;
    align-items: center;
    padding: 0 16px;
  }

  .header-logo {
    height: 24px;
    width: auto;
  }

  .logo-path {
    fill: white;
  }

  @media (prefers-color-scheme: dark) {
    .tabs {
      background: #2c2c2e;
      border-bottom-color: rgba(255, 255, 255, 0.1);
    }
  }

  .tab {
    padding: 0.75rem 1.5rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: rgba(0, 0, 0, 0.6);
    cursor: pointer;
    font-size: 13px;
    font-weight: 400;
    letter-spacing: -0.01em;
    transition: all 0.15s ease-out;
  }

  @media (prefers-color-scheme: dark) {
    .tab {
      color: rgba(255, 255, 255, 0.6);
    }
  }

  .tab:hover {
    color: rgba(0, 0, 0, 0.85);
    background: rgba(0, 0, 0, 0.05);
  }

  @media (prefers-color-scheme: dark) {
    .tab:hover {
      color: rgba(255, 255, 255, 0.85);
      background: rgba(255, 255, 255, 0.05);
    }
  }

  .tab.active {
    color: rgba(0, 0, 0, 0.95);
    border-bottom-color: #007AFF;
  }

  @media (prefers-color-scheme: dark) {
    .tab.active {
      color: rgba(255, 255, 255, 0.95);
    }
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    background: transparent;
  }

  .tab-content {
    max-width: 1000px;
    margin: 0 auto;
  }

  h2 {
    margin: 0 0 20px 0;
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.022em;
    color: rgba(0, 0, 0, 0.95);
  }

  @media (prefers-color-scheme: dark) {
    h2 {
      color: rgba(255, 255, 255, 0.95);
    }
  }

  .message {
    position: fixed;
    top: 20px;
    left: 50%;
    transform: translateX(-50%);
    padding: 10px 16px;
    font-size: 13px;
    border-radius: 8px;
    z-index: 1000;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    max-width: 90%;
    pointer-events: none;
  }

  .error {
    color: white;
    background: rgba(255, 69, 58, 0.95);
    border: 0.5px solid rgba(255, 69, 58, 0.3);
  }

  .success {
    color: white;
    background: rgba(48, 209, 88, 0.95);
    border: 0.5px solid rgba(48, 209, 88, 0.3);
  }

  .form-group {
    margin-bottom: 20px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    font-size: 13px;
    font-weight: 400;
    color: rgba(255, 255, 255, 0.7);
    letter-spacing: -0.01em;
  }

  .folder-selector {
    display: flex;
    gap: 8px;
  }

  .folder-selector input {
    flex: 1;
    padding: 6px 10px;
    font-size: 13px;
    background: rgba(255, 255, 255, 0.1);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.9);
    border-radius: 6px;
    transition: all 0.15s ease-out;
  }

  .folder-selector input:focus {
    outline: none;
    border-color: #007AFF;
    background: rgba(255, 255, 255, 0.15);
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.2);
  }

  .form-group select {
    padding: 6px 10px;
    font-size: 13px;
    min-width: 200px;
    background: rgba(255, 255, 255, 0.1);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.9);
    border-radius: 6px;
    transition: all 0.15s ease-out;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg width='12' height='8' viewBox='0 0 12 8' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1L6 6L11 1' stroke='rgba(255,255,255,0.7)' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    padding-right: 28px;
  }

  .form-group select:focus {
    outline: none;
    border-color: #007AFF;
    background-color: rgba(255, 255, 255, 0.15);
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.2);
  }

  .org-mode-group {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .org-mode-group label {
    margin-bottom: 0;
    flex-shrink: 0;
  }

  .org-mode-group select {
    flex-shrink: 0;
  }

  .mode-description {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.6);
    letter-spacing: -0.01em;
  }

  @media (prefers-color-scheme: light) {
    .mode-description {
      color: rgba(0, 0, 0, 0.6);
    }
  }

  .form-group button {
    padding: 6px 16px;
    font-size: 13px;
    font-weight: 500;
    margin-right: 8px;
    background: #007AFF;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease-out;
    letter-spacing: -0.01em;
  }

  .form-group button:hover:not(:disabled) {
    background: #0051D5;
    transform: translateY(-0.5px);
  }

  .form-group button:active:not(:disabled) {
    transform: translateY(0);
  }

  .form-group button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: rgba(0, 0, 0, 0.7);
    margin-top: 8px;
  }

  @media (prefers-color-scheme: dark) {
    .status-indicator {
      color: rgba(255, 255, 255, 0.7);
    }
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.3);
    flex-shrink: 0;
  }

  .dot.active {
    background: #30D158;
    box-shadow: 0 0 8px rgba(48, 209, 88, 0.5);
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 16px;
  }

  .rule-item {
    border-bottom: 0.5px solid rgba(0, 0, 0, 0.1);
    padding-bottom: 5px;
  }

  @media (prefers-color-scheme: dark) {
    .rule-item {
      border-bottom-color: rgba(255, 255, 255, 0.1);
    }
  }

  .rule-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 0;
    cursor: pointer;
  }

  .rule-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }

  .collapse-btn {
    color: rgba(0, 0, 0, 0.6);
    font-size: 11px;
    transition: transform 0.2s ease-out;
    pointer-events: none;
  }

  @media (prefers-color-scheme: dark) {
    .collapse-btn {
      color: rgba(255, 255, 255, 0.6);
    }
  }

  .collapse-btn.collapsed {
    transform: rotate(-90deg);
  }

  .rule-name {
    flex: 1;
    background: #ffffff;
    border: 0.5px solid rgba(0, 0, 0, 0.2);
    color: rgba(0, 0, 0, 0.9);
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 13px;
    min-width: 150px;
    transition: all 0.15s ease-out;
  }

  @media (prefers-color-scheme: dark) {
    .rule-name {
      background: #3a3a3c;
      border-color: rgba(255, 255, 255, 0.2);
      color: rgba(255, 255, 255, 0.9);
    }
  }

  .rule-name:focus {
    outline: none;
    border-color: #007AFF;
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.2);
  }

  .remove-btn {
    background: transparent;
    color: #FF453A;
    border: 0.5px solid rgba(255, 69, 58, 0.3);
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.15s ease-out;
  }

  .remove-btn:hover {
    background: rgba(255, 69, 58, 0.15);
    border-color: rgba(255, 69, 58, 0.5);
  }

  .rule-content {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-left: 20px;
    margin-top: 12px;
  }

  .condition-group {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .condition-group label {
    min-width: 100px;
    font-size: 13px;
    color: rgba(0, 0, 0, 0.7);
    margin: 0;
  }

  @media (prefers-color-scheme: dark) {
    .condition-group label {
      color: rgba(255, 255, 255, 0.7);
    }
  }

  .condition-group select,
  .condition-group input {
    flex: 1;
    padding: 6px 10px;
    border: 0.5px solid rgba(0, 0, 0, 0.2);
    border-radius: 6px;
    font-size: 13px;
    background: #ffffff;
    color: rgba(0, 0, 0, 0.9);
    transition: all 0.15s ease-out;
  }

  @media (prefers-color-scheme: dark) {
    .condition-group select,
    .condition-group input {
      border-color: rgba(255, 255, 255, 0.2);
      background: #3a3a3c;
      color: rgba(255, 255, 255, 0.9);
    }
  }

  .condition-group select:focus,
  .condition-group input:focus {
    outline: none;
    border-color: #007AFF;
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.2);
  }

  .condition-group select {
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg width='12' height='8' viewBox='0 0 12 8' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1L6 6L11 1' stroke='rgba(0,0,0,0.6)' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    padding-right: 28px;
  }

  @media (prefers-color-scheme: dark) {
    .condition-group select {
      background-image: url("data:image/svg+xml,%3Csvg width='12' height='8' viewBox='0 0 12 8' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1L6 6L11 1' stroke='rgba(255,255,255,0.7)' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    }
  }

  .condition-group .folder-selector {
    flex: 1;
  }

  .rules-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .rules-header h2 {
    margin: 0;
  }

  .add-rule-btn {
    padding: 6px 12px;
    background: #007AFF;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease-out;
    letter-spacing: -0.01em;
  }

  .add-rule-btn:hover {
    background: #0051D5;
    transform: translateY(-0.5px);
  }

  .add-rule-btn:active {
    transform: translateY(0);
  }

  .no-rules {
    text-align: center;
    padding: 40px;
    color: rgba(0, 0, 0, 0.6);
    font-size: 13px;
  }

  @media (prefers-color-scheme: dark) {
    .no-rules {
      color: rgba(255, 255, 255, 0.6);
    }
  }

  .empty-state {
    text-align: center;
    padding: 40px;
    color: rgba(0, 0, 0, 0.6);
    font-size: 13px;
  }

  @media (prefers-color-scheme: dark) {
    .empty-state {
      color: rgba(255, 255, 255, 0.6);
    }
  }

  .files-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .file-item {
    border-bottom: 0.5px solid rgba(0, 0, 0, 0.1);
    padding-bottom: 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  @media (prefers-color-scheme: dark) {
    .file-item {
      border-bottom-color: rgba(255, 255, 255, 0.1);
    }
  }

  .file-info {
    flex: 1;
    min-width: 0;
  }

  .file-name {
    font-weight: 500;
    color: rgba(0, 0, 0, 0.9);
    margin-bottom: 4px;
    word-break: break-all;
    font-size: 13px;
    letter-spacing: -0.01em;
  }

  @media (prefers-color-scheme: dark) {
    .file-name {
      color: rgba(255, 255, 255, 0.9);
    }
  }

  .file-details {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: rgba(0, 0, 0, 0.6);
    flex-wrap: wrap;
  }

  @media (prefers-color-scheme: dark) {
    .file-details {
      color: rgba(255, 255, 255, 0.6);
    }
  }

  .file-extension {
    text-transform: uppercase;
    font-weight: 500;
  }

  .file-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .select-btn {
    background: #007AFF;
    color: white;
    border: none;
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: all 0.15s ease-out;
    letter-spacing: -0.01em;
  }

  .select-btn:hover {
    background: #0051D5;
    transform: translateY(-0.5px);
  }

  .select-btn:active {
    transform: translateY(0);
  }

  .skip-btn {
    background: transparent;
    color: rgba(0, 0, 0, 0.7);
    border: 0.5px solid rgba(0, 0, 0, 0.2);
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.15s ease-out;
  }

  @media (prefers-color-scheme: dark) {
    .skip-btn {
      color: rgba(255, 255, 255, 0.7);
      border-color: rgba(255, 255, 255, 0.2);
    }
  }

  .skip-btn:hover {
    background: rgba(0, 0, 0, 0.05);
    border-color: rgba(0, 0, 0, 0.3);
  }

  @media (prefers-color-scheme: dark) {
    .skip-btn:hover {
      background: rgba(255, 255, 255, 0.1);
      border-color: rgba(255, 255, 255, 0.3);
    }
  }
</style>
