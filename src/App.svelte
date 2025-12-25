<script>
  import ConfigPanel from './ConfigPanel.svelte';
  import StatusPanel from './StatusPanel.svelte';
  import PendingFilesPanel from './PendingFilesPanel.svelte';

  let error = '';
  let success = '';

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
</script>

<main>
  <h1>Folder Watcher</h1>
  
  {#if error}
    <div class="message error">{error}</div>
  {/if}
  
  {#if success}
    <div class="message success">{success}</div>
  {/if}

  <StatusPanel onError={handleError} onSuccess={handleSuccess} />
  <PendingFilesPanel onError={handleError} onSuccess={handleSuccess} />
  <ConfigPanel onError={handleError} onSuccess={handleSuccess} />
</main>

<style>
  main {
    max-width: 900px;
    margin: 0 auto;
    color: #e0e0e0;
  }

  h1 {
    margin-bottom: 2rem;
    font-size: 2rem;
    color: #e0e0e0;
  }

  .message {
    padding: 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  .error {
    background-color: #4a1a1a;
    color: #ff6b6b;
    border: 1px solid #6a2a2a;
  }

  .success {
    background-color: #1a4a1a;
    color: #6bff6b;
    border: 1px solid #2a6a2a;
  }
</style>
