<script>
  export let version = '0.1.0';
</script>

<header>
  <h1>🔧 Rivet</h1>
  <span class="version">v{version}</span>
</header>

<style>
  header {
    background: #161b22;
    padding: 16px 24px;
    border-bottom: 1px solid #30363d;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }

  header h1 {
    font-size: 20px;
    font-weight: 600;
    color: #f0f6fc;
  }

  header .version {
    font-size: 14px;
    color: #8b949e;
  }
</style>