<script>
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  export let onError;
  export let onSuccess;

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

  let config = { watched_folder: null, rules: [] };
  let rules = [];
  let collapsedRules = {};
  let editingName = null;

  function toggleCollapse(ruleId) {
    console.log('Toggle collapse for rule:', ruleId, 'Current state:', collapsedRules[ruleId]);
    collapsedRules[ruleId] = !collapsedRules[ruleId];
    collapsedRules = collapsedRules;
    console.log('New state:', collapsedRules[ruleId]);
  }

  onMount(async () => {
    await loadConfig();
  });

  async function loadConfig() {
    try {
      config = await invoke('get_config');
      if (!config.rules) {
        config.rules = [];
      }
      rules = config.rules.map((rule, index) => ({
        id: Date.now() + index, // Use unique IDs
        name: rule.name || `Rule ${index + 1}`,
        conditionType: getConditionType(rule.condition),
        conditionValue: getConditionValue(rule.condition),
        operator: getOperator(rule.condition),
        destination: rule.destination
      }));
      // Reset collapsed state when loading
      collapsedRules = {};
    } catch (err) {
      onError(`Failed to load config: ${err}`);
    }
  }

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
    saveConfig();
  }

  function removeRule(index) {
    const ruleId = rules[index].id;
    rules = rules.filter((_, i) => i !== index);
    const newCollapsed = { ...collapsedRules };
    delete newCollapsed[ruleId];
    collapsedRules = newCollapsed;
    saveConfig();
  }


  function isCollapsed(ruleId) {
    return !!collapsedRules[ruleId];
  }

  async function selectDestinationFolder(ruleIndex) {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
      });
      
      if (selected) {
        const folder = Array.isArray(selected) ? selected[0] : selected;
        rules[ruleIndex].destination = folder;
        await saveConfig();
      }
    } catch (err) {
      onError(`Failed to select folder: ${err}`);
    }
  }

  async function saveConfig() {
    try {
      config.rules = rules.map(rule => {
        let condition;
        if (rule.conditionType === 'filetype') {
          condition = { type: 'filetype', value: rule.conditionValue };
        } else if (rule.conditionType === 'name') {
          condition = { type: 'name', pattern: rule.conditionValue };
        } else if (rule.conditionType === 'created_date') {
          condition = { 
            type: 'created_date', 
            operator: rule.operator,
            value: rule.conditionValue 
          };
        }
        return {
          name: rule.name || null,
          condition,
          destination: rule.destination
        };
      });
      
      await invoke('save_config', { config });
      onSuccess('Configuration saved');
    } catch (err) {
      onError(`Failed to save config: ${err}`);
    }
  }
</script>

<section class="config-panel">
  <h2>Organization Rules</h2>
  
  <p class="description">
    Create rules to automatically organize files. Rules are evaluated in order, and the first matching rule is used.
  </p>

  <div class="rules-list">
    {#each rules as rule, index (rule.id)}
      <div class="rule-item">
        <div class="rule-header" on:click={() => toggleCollapse(rule.id)}>
          <div class="rule-header-left">
            <div 
              class="collapse-btn" 
              class:collapsed={collapsedRules[rule.id]}
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M4 2L8 6L4 10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
            <input 
              type="text" 
              class="rule-name"
              bind:value={rule.name}
              on:click|stopPropagation
              on:focus={() => editingName = rule.id}
              on:blur={() => { editingName = null; saveConfig(); }}
              on:keydown|stopPropagation={(e) => {
                if (e.key === 'Enter') {
                  e.target.blur();
                }
              }}
              placeholder="Rule name..."
            />
          </div>
          <button class="remove-btn" on:click|stopPropagation={() => removeRule(index)}>Remove</button>
        </div>
        
        {#if !collapsedRules[rule.id]}
          <div class="rule-content">
            <div class="condition-group">
              <label>If</label>
              <select bind:value={rule.conditionType} on:change={saveConfig}>
                {#each conditionTypes as type}
                  <option value={type.value}>{type.label}</option>
                {/each}
              </select>
            </div>

            {#if rule.conditionType === 'created_date'}
              <div class="condition-group">
                <select bind:value={rule.operator} on:change={saveConfig}>
                  {#each dateOperators as op}
                    <option value={op.value}>{op.label}</option>
                  {/each}
                </select>
              </div>
            {/if}

            <div class="condition-group">
              {#if rule.conditionType === 'filetype'}
                <select bind:value={rule.conditionValue} on:change={saveConfig}>
                  <option value="">Select file type...</option>
                  {#each commonFileTypes as fileType}
                    <option value={fileType}>{fileType}</option>
                  {/each}
                </select>
              {:else if rule.conditionType === 'name'}
                <input 
                  type="text" 
                  bind:value={rule.conditionValue}
                  placeholder="e.g., invoice*, report*.pdf"
                  on:input={saveConfig}
                />
              {:else if rule.conditionType === 'created_date'}
                <input 
                  type="date" 
                  bind:value={rule.conditionValue}
                  on:change={saveConfig}
                />
              {/if}
            </div>

            <div class="condition-group">
              <label>Then move to:</label>
              <div class="folder-selector">
                <input 
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
</section>

<style>
  .config-panel {
    background: #2d2d2d;
    padding: 1.5rem;
    border-radius: 8px;
    border: 1px solid #444;
  }

  h2 {
    margin-bottom: 0.5rem;
    font-size: 1.5rem;
    color: #e0e0e0;
  }

  .description {
    color: #aaa;
    margin-bottom: 1.5rem;
    font-size: 0.9em;
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .rule-item {
    background: #1a1a1a;
    border-radius: 8px;
    padding: 0;
    border: 1px solid #444;
    overflow: hidden;
  }

  .rule-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    user-select: none;
    background: #252525;
    border-bottom: 1px solid #444;
    transition: background-color 0.2s;
    cursor: pointer;
  }

  .rule-header:hover {
    background: #2a2a2a;
  }

  .rule-header-left {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
  }

  .collapse-btn {
    background: transparent;
    border: none;
    padding: 0.25em;
    color: #aaa;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s;
    min-width: 24px;
    height: 24px;
    flex-shrink: 0;
    pointer-events: none;
  }

  .collapse-btn:not(.collapsed) {
    transform: rotate(90deg);
  }
  
  .collapse-btn.collapsed {
    transform: rotate(0deg);
  }

  .rule-name {
    flex: 1;
    background: #2d2d2d;
    border: 1px solid #444;
    color: #e0e0e0;
    padding: 0.4em 0.6em;
    border-radius: 4px;
    font-size: 1em;
    font-weight: 500;
    min-width: 150px;
  }

  .rule-name:focus {
    outline: none;
    border-color: #4a90e2;
  }

  .remove-btn {
    background: #4a1a1a;
    color: #ff6b6b;
    border: 1px solid #6a2a2a;
    padding: 0.4em 0.8em;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9em;
  }

  .remove-btn:hover {
    background: #5a2a2a;
  }

  .rule-content {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
  }

  .condition-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .condition-group label {
    min-width: 80px;
    font-weight: 500;
    color: #e0e0e0;
  }

  .condition-group select,
  .condition-group input[type="text"],
  .condition-group input[type="date"] {
    flex: 1;
    padding: 0.5em;
    border: 1px solid #444;
    border-radius: 4px;
    font-size: 1em;
    background: #2d2d2d;
    color: #e0e0e0;
  }

  .condition-group select:focus,
  .condition-group input[type="text"]:focus,
  .condition-group input[type="date"]:focus {
    outline: none;
    border-color: #4a90e2;
  }

  .folder-selector {
    display: flex;
    gap: 0.5rem;
    flex: 1;
  }

  .folder-selector input {
    flex: 1;
  }

  .folder-selector button {
    padding: 0.5em 1em;
    background: #4a90e2;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .folder-selector button:hover {
    background: #357abd;
  }

  .add-rule-btn {
    width: 100%;
    padding: 0.75em;
    background: #4a90e2;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 1em;
    cursor: pointer;
    font-weight: 500;
  }

  .add-rule-btn:hover {
    background: #357abd;
  }

  .no-rules {
    text-align: center;
    padding: 2rem;
    color: #999;
    background: #1a1a1a;
    border-radius: 8px;
    border: 1px dashed #444;
  }
</style>
