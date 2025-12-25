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
      isWatching = watchedFolder !== '';
      organizationMode = await invoke('get_organization_mode');
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

  async function stopWatching() {
    try {
      await invoke('stop_watching');
      isWatching = false;
      handleSuccess('Stopped watching folder');
    } catch (err) {
      handleError(`Failed to stop watching: ${err}`);
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
      collapsedRules = {};
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
    collapsedRules[newId] = false;
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
    <div class="tabs">
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

    <div class="content">
      {#if error}
        <div class="message error">{error}</div>
      {/if}
      
      {#if success}
        <div class="message success">{success}</div>
      {/if}

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
            <label for="org-mode">Organization Mode:</label>
            <select id="org-mode" bind:value={organizationMode} on:change={changeOrganizationMode}>
              <option value="auto">Auto</option>
              <option value="ask">Ask</option>
              <option value="both">Both</option>
            </select>
          </div>

          <div class="form-group">
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
        </div>
      {:else if activeTab === 'rules'}
        <div class="tab-content">
          <h2>Organization Rules</h2>
          
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

          <button class="add-rule-btn" on:click={addRule}>+ Add Rule</button>
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
    background: #1a1a1a;
    color: #e0e0e0;
    overflow: hidden;
  }

  .tabs {
    display: flex;
    background: #1a1a1a;
    border-bottom: 1px solid #333;
    padding: 0;
    flex-shrink: 0;
  }

  .tab {
    padding: 0.75rem 1.5rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #999;
    cursor: pointer;
    font-size: 0.9em;
    transition: all 0.2s;
  }

  .tab:hover {
    color: #e0e0e0;
  }

  .tab.active {
    color: #e0e0e0;
    border-bottom-color: #4a90e2;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
    background: #1a1a1a;
  }

  .tab-content {
    max-width: 1000px;
    margin: 0 auto;
  }

  h2 {
    margin: 0 0 1.5rem 0;
    font-size: 1rem;
    font-weight: 500;
    color: #e0e0e0;
  }

  .message {
    padding: 0.75rem;
    margin-bottom: 1rem;
    font-size: 0.9em;
  }

  .error {
    color: #ff6b6b;
  }

  .success {
    color: #6bff6b;
  }

  .form-group {
    margin-bottom: 1.5rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-size: 0.9em;
    color: #999;
  }

  .folder-selector {
    display: flex;
    gap: 0.5rem;
  }

  .folder-selector input {
    flex: 1;
    padding: 0.5rem;
    font-size: 0.9em;
    background: #252525;
    border: 1px solid #333;
    color: #e0e0e0;
    border-radius: 3px;
  }

  .form-group select {
    padding: 0.5rem;
    font-size: 0.9em;
    min-width: 200px;
    background: #252525;
    border: 1px solid #333;
    color: #e0e0e0;
    border-radius: 3px;
  }

  .form-group button {
    padding: 0.5rem 1rem;
    font-size: 0.9em;
    margin-right: 0.5rem;
    background: #4a90e2;
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  .form-group button:hover:not(:disabled) {
    background: #357abd;
  }

  .form-group button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9em;
    color: #999;
    margin-top: 0.5rem;
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

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .rule-item {
    border-bottom: 1px solid #333;
    padding-bottom: 1rem;
  }

  .rule-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    cursor: pointer;
  }

  .rule-header-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
  }

  .collapse-btn {
    color: #999;
    font-size: 0.8em;
    transition: transform 0.2s;
    pointer-events: none;
  }

  .collapse-btn.collapsed {
    transform: rotate(-90deg);
  }

  .rule-name {
    flex: 1;
    background: #252525;
    border: 1px solid #333;
    color: #e0e0e0;
    padding: 0.4rem 0.6rem;
    border-radius: 3px;
    font-size: 0.9em;
    min-width: 150px;
  }

  .remove-btn {
    background: transparent;
    color: #ff6b6b;
    border: 1px solid #6a2a2a;
    padding: 0.4rem 0.75rem;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.85em;
  }

  .remove-btn:hover {
    background: #4a1a1a;
  }

  .rule-content {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-left: 1.5rem;
  }

  .condition-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .condition-group label {
    min-width: 100px;
    font-size: 0.9em;
    color: #999;
    margin: 0;
  }

  .condition-group select,
  .condition-group input {
    flex: 1;
    padding: 0.5rem;
    border: 1px solid #333;
    border-radius: 3px;
    font-size: 0.9em;
    background: #252525;
    color: #e0e0e0;
  }

  .condition-group .folder-selector {
    flex: 1;
  }

  .add-rule-btn {
    padding: 0.6rem 1rem;
    background: #4a90e2;
    color: white;
    border: none;
    border-radius: 3px;
    font-size: 0.9em;
    cursor: pointer;
  }

  .add-rule-btn:hover {
    background: #357abd;
  }

  .no-rules {
    text-align: center;
    padding: 2rem;
    color: #999;
    font-size: 0.9em;
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: #999;
    font-size: 0.9em;
  }

  .files-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .file-item {
    border-bottom: 1px solid #333;
    padding-bottom: 1rem;
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
    color: #999;
    border: 1px solid #333;
    padding: 0.4rem 0.75rem;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.85em;
  }

  .skip-btn:hover {
    background: #252525;
  }
</style>
