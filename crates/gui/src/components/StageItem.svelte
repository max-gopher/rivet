<script lang="ts">
    import type { StageResult } from '../lib/api';

    export let stage: StageResult;

    let showRequest = false;
    let showResponse = false;
</script>

<div class="stage-item">
    <span class="stage-name">{stage.name}</span>

    <div class="stage-status">
        <span class="status-badge {stage.passed ? 'passed' : 'failed'}">
            {stage.passed ? '✅ Пройден' : '❌ Провален'}
        </span>

        <span class="duration">{stage.duration_ms}ms</span>

        {#if stage.status}
            <span class="status-code">{stage.status}</span>
        {/if}
    </div>

    {#if stage.error}
        <div class="stage-error">{stage.error}</div>
    {/if}

    <!-- Кнопка показа запроса -->
    <button class="detail-toggle" on:click={() => showRequest = !showRequest}>
        {showRequest ? '▼' : '▶'} Запрос
    </button>
    {#if showRequest}
        <div class="detail-content">
            <div class="detail-row">
                <span class="detail-label">Метод:</span>
                <span class="detail-value">{stage.request.method}</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">URL:</span>
                <span class="detail-value">{stage.request.url}</span>
            </div>
            {#if Object.keys(stage.request.params).length > 0}
                <div class="detail-row">
                    <span class="detail-label">Параметры:</span>
                    <pre class="detail-json">{JSON.stringify(stage.request.params, null, 2)}</pre>
                </div>
            {/if}
            <div class="detail-row">
                <span class="detail-label">Заголовки:</span>
                <pre class="detail-json">{JSON.stringify(stage.request.headers, null, 2)}</pre>
            </div>
            {#if stage.request.body}
                <div class="detail-row">
                    <span class="detail-label">Тело:</span>
                    <pre class="detail-json">{JSON.stringify(stage.request.body, null, 2)}</pre>
                </div>
            {/if}
        </div>
    {/if}

    <!-- Кнопка показа ответа -->
    <button class="detail-toggle" on:click={() => showResponse = !showResponse}>
        {showResponse ? '▼' : '▶'} Ответ
    </button>
    {#if showResponse}
        <div class="detail-content">
            <div class="detail-row">
                <span class="detail-label">Статус:</span>
                <span class="detail-value">{stage.response.status}</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">Заголовки:</span>
                <pre class="detail-json">{JSON.stringify(stage.response.headers, null, 2)}</pre>
            </div>
            <div class="detail-row">
                <span class="detail-label">Тело:</span>
                <pre class="detail-body">{stage.response.body}</pre>
            </div>
        </div>
    {/if}
</div>

<style>
    .stage-item {
        background: #161b22;
        border: 1px solid #30363d;
        border-radius: 6px;
        padding: 12px 16px;
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 8px 16px;
    }

    .stage-name {
        font-size: 14px;
        font-weight: 500;
        flex: 1;
        min-width: 150px;
        color: #f0f6fc;
    }

    .stage-status {
        display: flex;
        align-items: center;
        gap: 12px;
        flex-wrap: wrap;
    }

    .status-badge {
        padding: 2px 10px;
        border-radius: 12px;
        font-size: 12px;
        font-weight: 600;
    }

    .status-badge.passed {
        background: #0f2d16;
        color: #3fb950;
    }

    .status-badge.failed {
        background: #2d0f0f;
        color: #f85149;
    }

    .duration {
        font-size: 12px;
        color: #8b949e;
    }

    .status-code {
        font-size: 12px;
        color: #58a6ff;
        font-family: monospace;
    }

    .stage-error {
        width: 100%;
        font-size: 13px;
        color: #f85149;
        padding: 4px 8px;
        background: #0d1117;
        border-radius: 4px;
        font-family: monospace;
        word-break: break-all;
        border: 1px solid rgba(248, 81, 73, 0.2);
    }

    .detail-toggle {
        background: none;
        border: 1px solid #30363d;
        border-radius: 4px;
        color: #8b949e;
        cursor: pointer;
        font-size: 12px;
        padding: 4px 10px;
        transition: all 0.2s;
        width: 100%;
        text-align: left;
        margin-top: 4px;
    }

    .detail-toggle:hover {
        background: #21262d;
        border-color: #58a6ff;
        color: #f0f6fc;
    }

    .detail-content {
        background: #0d1117;
        border-radius: 4px;
        padding: 8px 12px;
        margin-top: 4px;
        font-size: 13px;
        width: 100%;
        overflow-x: auto;
    }

    .detail-row {
        margin-bottom: 4px;
    }

    .detail-label {
        color: #8b949e;
        font-weight: 600;
        margin-right: 8px;
    }

    .detail-value {
        color: #f0f6fc;
        word-break: break-all;
    }

    .detail-json {
        color: #f0f6fc;
        font-family: 'Courier New', monospace;
        font-size: 12px;
        margin: 4px 0;
        padding: 4px 8px;
        background: #161b22;
        border-radius: 4px;
        overflow-x: auto;
        white-space: pre-wrap;
        word-break: break-word;
    }

    .detail-body {
        color: #f0f6fc;
        font-family: 'Courier New', monospace;
        font-size: 12px;
        margin: 4px 0;
        padding: 4px 8px;
        background: #161b22;
        border-radius: 4px;
        max-height: 300px;
        overflow-y: auto;
        white-space: pre-wrap;
        word-break: break-word;
    }
</style>