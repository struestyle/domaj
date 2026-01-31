<script>
  import "../app.css";

  async function triggerScan() {
    try {
      const response = await fetch("/api/scan", { method: "POST" });
      if (response.ok) {
        alert("Scan lancé en arrière-plan");
      } else {
        alert("Erreur lors du lancement du scan");
      }
    } catch (error) {
      console.error("Scan error:", error);
      alert("Erreur de connexion au serveur");
    }
  }
</script>

<div class="app">
  <nav class="navbar">
    <div class="container navbar-content">
      <a href="/" class="logo">
        <span class="logo-icon">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path
              d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
            ></path>
            <polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline>
            <line x1="12" y1="22.08" x2="12" y2="12"></line>
          </svg>
        </span>
        <span class="logo-text">Domaj</span>
      </a>

      <div class="nav-links">
        <a href="/" class="nav-link">Dashboard</a>
        <a href="/servers" class="nav-link">Serveurs</a>
      </div>

      <div class="nav-actions">
        <button class="btn btn-primary" on:click={triggerScan}>
          <svg
            class="btn-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
          </svg>
          Scanner
        </button>
      </div>
    </div>
  </nav>

  <main class="main-content">
    <slot />
  </main>

  <footer class="footer">
    <div class="container">
      <p class="text-muted text-sm">Domaj v1.0.0 - Docker Mise à Jour</p>
    </div>
  </footer>
</div>

<style>
  .app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .navbar {
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    padding: var(--spacing-md) 0;
    position: sticky;
    top: 0;
    z-index: 100;
    backdrop-filter: blur(10px);
  }

  .navbar-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .logo-icon {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
  }

  .logo-icon svg {
    width: 28px;
    height: 28px;
    stroke: var(--color-primary);
  }

  .logo-text {
    background: linear-gradient(
      135deg,
      var(--color-primary) 0%,
      var(--color-secondary) 100%
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .btn-icon {
    width: 16px;
    height: 16px;
    margin-right: 6px;
    vertical-align: middle;
  }

  .nav-links {
    display: flex;
    gap: var(--spacing-lg);
  }

  .nav-link {
    color: var(--text-secondary);
    font-weight: 500;
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-sm);
    transition: all var(--transition-fast);
  }

  .nav-link:hover {
    color: var(--text-primary);
    background: var(--bg-card);
  }

  .main-content {
    flex: 1;
    padding: var(--spacing-xl) 0;
  }

  .footer {
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
    padding: var(--spacing-lg) 0;
    text-align: center;
  }

  @media (max-width: 768px) {
    .nav-links {
      display: none;
    }
  }
</style>
