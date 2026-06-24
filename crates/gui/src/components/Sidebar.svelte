<script lang="ts">
  export let configName: string = '';
  export let isRunning: boolean = false;
  export let error: string | null = null;
  export let onRun: () => void;
  export let onSelect: () => void;

  function handleClick() {
    console.log('Sidebar button clicked');
    onSelect();
  }
</script>

<aside class="sidebar">
  <h3>📁 Конфигурация</h3>

  <button on:click={handleClick} class="btn-select">
    📁 Выбрать файл
  </button>

  <button
    class="btn-run"
    disabled={!configName || isRunning}
    on:click={onRun}
  >
    {#if isRunning}
      ⏳ Запуск...
    {:else}
      ▶ Запустить тесты
    {/if}
  </button>

  {#if error}
    <div class="error-message">
      ❌ {error}
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    width: 300px;
    background: #161b22;
    border-right: 1px solid #30363d;
    padding: 16px;
    overflow-y: auto;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .sidebar h3 {
    font-size: 14px;
    font-weight: 600;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0;
  }

  .btn-run {
    width: 100%;
    padding: 10px;
    background: #238636;
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .btn-run:hover:not(:disabled) {
    background: #2ea043;
  }

  .btn-run:disabled {
    background: #21262d;
    color: #484f58;
    cursor: not-allowed;
  }

  .error-message {
    padding: 8px 12px;
    background: #2d0f0f;
    border: 1px solid #f85149;
    border-radius: 6px;
    color: #f85149;
    font-size: 13px;
    word-break: break-word;
  }
</style>