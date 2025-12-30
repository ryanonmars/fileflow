<script>
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import FileOrganizationModal from './FileOrganizationModal.svelte';
  import AboutModal from './AboutModal.svelte';
  import UpdateModal from './UpdateModal.svelte';
  import appIcon from './app-icon.png';

  let error = '';
  let success = '';
  let isModalWindow = false;
  let activeTab = 'general';

  // General tab state
  let watchedFolder = '';
  let isWatching = false;
  let config = null;
  let organizationMode = 'auto';
  let launchAtLogin = false;
  let autoCheckForUpdates = true;

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
  let showAbout = false;
  let aboutVersion = '';
  let aboutProductName = '';

  let isAboutWindow = false;
  let isUpdateWindow = false;

  onMount(async () => {
    const appWindow = getCurrentWindow();
    const label = appWindow.label;
    
    if (label === 'file-organization') {
      isModalWindow = true;
    } else if (label === 'about') {
      isAboutWindow = true;
    } else if (label === 'update') {
      isUpdateWindow = true;
    } else {
      appWindow.listen('tauri://close-requested', async () => {
        await appWindow.hide();
      });
      
      // Expose function for menu to call
      window.showAboutDialog = (version, productName) => {
        console.log('showAboutDialog called', version, productName);
        aboutVersion = version;
        aboutProductName = productName;
        showAbout = true;
      };
      
      // Also listen for event (backup)
      const unlistenAbout = await listen('show-about', (event) => {
        aboutVersion = event.payload.version;
        aboutProductName = event.payload.productName;
        showAbout = true;
      });
      
      // Load config and initialize
      try {
        await loadGeneralConfig();
        await loadRules();
        await loadPendingFiles();
        pollingInterval = setInterval(async () => {
          await loadPendingFiles();
        }, 2000);
      } catch (err) {
        console.error('Error during initialization:', err);
        handleError(`Initialization error: ${err}`);
      }
      
      // Cleanup function
      return async () => {
        await unlistenAbout();
        if (pollingInterval) {
          clearInterval(pollingInterval);
        }
      };
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
      
      // Handle watched_folder - JSON serializes Option<String> as string or null
      const rawFolder = config?.watched_folder;
      if (rawFolder && (typeof rawFolder === 'string')) {
        watchedFolder = rawFolder.trim();
      } else {
        watchedFolder = '';
      }
      
      // Force reactivity update
      watchedFolder = watchedFolder;
      
      organizationMode = await invoke('get_organization_mode');
      launchAtLogin = config?.launch_at_login === true;
      autoCheckForUpdates = config?.auto_check_for_updates !== false;
      
      // Automatically start watching if a folder is configured
      if (watchedFolder && watchedFolder.length > 0) {
        try {
          await invoke('start_watching', { watchedFolder });
          isWatching = true;
        } catch (err) {
          console.error('Failed to auto-start watching:', err);
          isWatching = false;
        }
      } else {
        isWatching = false;
      }
    } catch (err) {
      console.error('Error loading config:', err);
      handleError(`Failed to load config: ${err}`);
    }
  }

  async function saveGeneralSettings() {
    try {
      if (config) {
        config.launch_at_login = launchAtLogin;
        config.auto_check_for_updates = autoCheckForUpdates;
        config.watched_folder = watchedFolder || null;
        config.organization_mode = organizationMode;
        await invoke('save_config', { config });
        handleSuccess('Settings saved');
      }
    } catch (err) {
      handleError(`Failed to save settings: ${err}`);
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
        // Reload config to ensure we have latest values
        const currentConfig = await invoke('get_config');
        currentConfig.watched_folder = watchedFolder;
        currentConfig.organization_mode = organizationMode;
        await invoke('save_config', { config: currentConfig });
        await startWatching();
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
      handleSuccess('Cleared monitored folder');
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
{:else if isAboutWindow}
  <AboutModal />
{:else if isUpdateWindow}
  <UpdateModal />
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
        <img src={appIcon} alt="FileFlow" class="header-logo" />
      </div>
    </div>

    <div class="content">

      {#if activeTab === 'general'}
        <div class="tab-content">
          <h2 class="monitored-folder-title">Monitored Folder</h2>
          
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

          <div class="settings-section">
            <div class="setting-item">
              <label for="launch-at-login">Launch at login</label>
              <label class="toggle-switch">
                <input type="checkbox" id="launch-at-login" bind:checked={launchAtLogin} on:change={saveGeneralSettings} />
                <span class="toggle-slider"></span>
              </label>
            </div>
            <div class="setting-item">
              <label for="auto-check-updates">Automatically check for updates</label>
              <label class="toggle-switch">
                <input type="checkbox" id="auto-check-updates" bind:checked={autoCheckForUpdates} on:change={saveGeneralSettings} />
                <span class="toggle-slider"></span>
              </label>
            </div>
          </div>
        </div>
      {:else if activeTab === 'rules'}
        <div class="tab-content">
          <div class="rules-header">
            <h2 class="monitored-folder-title">Organization Rules</h2>
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
          <h2 class="monitored-folder-title">Pending Files</h2>
          
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

{#if showAbout}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div 
    class="about-overlay" 
    role="dialog" 
    aria-modal="true"
    aria-labelledby="about-title"
    tabindex="-1"
    on:click={() => showAbout = false} 
    on:keydown={(e) => { if (e.key === 'Escape') showAbout = false; }}
  >
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div class="about-dialog" role="document" on:click|stopPropagation>
      <img src={appIcon} alt="FileFlow" class="about-logo" />
      <h2 id="about-title">{aboutProductName || 'FileFlow'}</h2>
      <p class="about-version">Version {aboutVersion || '0.1.1'}</p>
      <button class="about-close" on:click={() => showAbout = false}>OK</button>
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

  .monitored-folder-title {
    font-size: 16px;
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

  .settings-section {
    margin-top: 32px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 8px 0;
    border: 0.5px solid rgba(255, 255, 255, 0.1);
  }

  @media (prefers-color-scheme: light) {
    .settings-section {
      background: rgba(0, 0, 0, 0.05);
      border-color: rgba(0, 0, 0, 0.1);
    }
  }

  .setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
  }

  .setting-item label:first-child {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.9);
    font-weight: 400;
    margin: 0;
    flex: 1;
  }

  @media (prefers-color-scheme: light) {
    .setting-item label:first-child {
      color: rgba(0, 0, 0, 0.9);
    }
  }

  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 26px;
    cursor: pointer;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(255, 255, 255, 0.2);
    transition: 0.2s;
    border-radius: 13px;
  }

  @media (prefers-color-scheme: light) {
    .toggle-slider {
      background-color: rgba(0, 0, 0, 0.2);
    }
  }

  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 20px;
    width: 20px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: 0.2s;
    border-radius: 50%;
  }

  .toggle-switch input:checked + .toggle-slider {
    background-color: #007AFF;
  }

  .toggle-switch input:checked + .toggle-slider:before {
    transform: translateX(18px);
  }

  .toggle-switch input:focus + .toggle-slider {
    box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.2);
  }

  .about-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
  }

  .about-dialog {
    background: rgba(30, 30, 30, 0.95);
    border-radius: 12px;
    padding: 32px;
    text-align: center;
    min-width: 300px;
    backdrop-filter: blur(20px);
    border: 0.5px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }

  @media (prefers-color-scheme: light) {
    .about-dialog {
      background: rgba(255, 255, 255, 0.95);
      border-color: rgba(0, 0, 0, 0.1);
    }
  }

  .about-logo {
    width: 64px;
    height: 64px;
    margin: 0 auto 16px;
    display: block;
  }

  .about-dialog h2 {
    margin: 0 0 8px 0;
    font-size: 24px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.9);
  }

  @media (prefers-color-scheme: light) {
    .about-dialog h2 {
      color: rgba(0, 0, 0, 0.9);
    }
  }

  .about-version {
    margin: 0 0 24px 0;
    font-size: 14px;
    color: rgba(255, 255, 255, 0.7);
  }

  @media (prefers-color-scheme: light) {
    .about-version {
      color: rgba(0, 0, 0, 0.7);
    }
  }

  .about-close {
    background: #007AFF;
    color: white;
    border: none;
    padding: 8px 24px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
  }

  .about-close:hover {
    background: #0056CC;
  }
</style>
