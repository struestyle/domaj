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
        <span class="logo-icon">🐳</span>
        <span class="logo-text">Domaj</span>
      </a>

      <div class="nav-links">
        <a href="/" class="nav-link">Dashboard</a>
        <a href="/servers" class="nav-link">Serveurs</a>
        <a href="/updates" class="nav-link">Mises à jour</a>
      </div>

      <div class="nav-actions">
        <button class="btn btn-primary" on:click={triggerScan}>
          <span>🔍</span>
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
    font-size: 2rem;
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
