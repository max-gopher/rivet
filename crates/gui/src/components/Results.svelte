<script lang="ts">
  import type { TestRunResult } from '../lib/api';
  import StageItem from './StageItem.svelte';

  export let results: TestRunResult;
</script>

<div class="results">
  <!-- Сводка -->
  <div class="summary">
    <div class="summary-grid">
      <div class="summary-item total">
        <div class="value">{results.total}</div>
        <div class="label">Всего</div>
      </div>
      <div class="summary-item passed">
        <div class="value">{results.passed_count}</div>
        <div class="label">✅ Пройдено</div>
      </div>
      <div class="summary-item failed">
        <div class="value">{results.failed_count}</div>
        <div class="label">❌ Провалено</div>
      </div>
      <div class="summary-item duration">
        <div class="value">{results.duration_ms}ms</div>
        <div class="label">⏱ Длительность</div>
      </div>
    </div>

    <div class="summary-status {results.passed ? 'passed' : 'failed'}">
      {results.passed ? '✅ Все тесты пройдены!' : '❌ Есть проваленные тесты'}
    </div>
  </div>

  <!-- Список этапов -->
  <div class="stage-list">
    {#each results.stages as stage}
      <StageItem {stage} />
    {:else}
      <div class="empty-stages">Нет этапов для отображения</div>
    {/each}
  </div>
</div>

<style>
  .results {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* ===== СВОДКА ===== */
  .summary {
    background: #161b22;
    border-radius: 8px;
    padding: 20px 24px;
    border: 1px solid #30363d;
  }

  .summary-grid {
    display: flex;
    gap: 24px;
    flex-wrap: wrap;
  }

  .summary-item {
    text-align: center;
  }

  .summary-item .value {
    font-size: 28px;
    font-weight: 600;
  }

  .summary-item .label {
    font-size: 12px;
    color: #8b949e;
    margin-top: 4px;
  }

  .summary-item.passed .value {
    color: #3fb950;
  }

  .summary-item.failed .value {
    color: #f85149;
  }

  .summary-item.total .value {
    color: #f0f6fc;
  }

  .summary-item.duration .value {
    color: #58a6ff;
  }

  .summary-status {
    margin-top: 12px;
    font-size: 18px;
    font-weight: 600;
  }

  .summary-status.passed {
    color: #3fb950;
  }

  .summary-status.failed {
    color: #f85149;
  }

  /* ===== СПИСОК ЭТАПОВ ===== */
  .stage-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .empty-stages {
    text-align: center;
    color: #8b949e;
    padding: 20px;
  }
</style>