<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type TestRunResult, type AppInfo } from './lib/api';
  import Header from './components/Header.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Results from './components/Results.svelte';

  let configPath = '';
  let configName = '';
  let isRunning = false;
  let results: TestRunResult | null = null;
  let error: string | null = null;
  let info: AppInfo = { name: 'Rivet', version: '0.1.0', description: '' };

  onMount(async () => {
    try {
      info = await api.getInfo();
    } catch (err) {
      console.error('Failed to load info:', err);
    }
  });

  async function selectConfig() {
    console.log('selectConfig called');
    try {
      const filePath = await api.selectFile();
      console.log('filePath:', filePath);
      if (filePath) {
        configPath = filePath;
        configName = filePath.split('/').pop() || filePath;
        error = null;

        const suite = await api.loadConfig(filePath);
        console.log('Config loaded:', suite);
      }
    } catch (err) {
      console.error('Error:', err);
      error = String(err);
      configPath = '';
      configName = '';
    }
  }

  async function runTests() {
    if (!configPath) return;

    isRunning = true;
    error = null;
    results = null;

    try {
      results = await api.runTests();
      console.log('🔍 App.svelte: results received:', results);
    } catch (err) {
      error = String(err);
      console.error('🔍 App.svelte: error:', err);
    } finally {
      isRunning = false;
    }
  }
</script>

<main>
  <Header version={info.version} />

  <div class="container">
    <Sidebar
            {configName}
            {isRunning}
            {error}
            onSelect={selectConfig}
            onRun={runTests}
    />

    <div class="content">
      {#if results}
        <Results {results} />
      {:else}
        <div class="empty-state">
          <div class="icon">📄</div>
          <h2>Загрузите конфигурацию</h2>
          <p>Выберите YAML файл с описанием тестов</p>
        </div>
      {/if}
    </div>
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .container {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .content {
    flex: 1;
    padding: 24px;
    overflow-y: auto;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #8b949e;
  }

  .empty-state .icon {
    font-size: 48px;
    margin-bottom: 16px;
  }

  .empty-state h2 {
    font-size: 24px;
    font-weight: 400;
    color: #f0f6fc;
    margin-bottom: 8px;
  }

  .empty-state p {
    font-size: 16px;
  }
</style>