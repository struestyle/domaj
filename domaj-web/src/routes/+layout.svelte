<script>
  import "../app.css";
  import Toast from "$lib/components/Toast.svelte";
  import { toasts } from "$lib/stores/toast.js";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import {
    isAuthenticated,
    user,
    logout,
    checkAuth,
    getToken,
  } from "$lib/stores/auth.js";

  $: isLoginPage = $page.url.pathname === "/login";

  onMount(async () => {
    if (!isLoginPage) {
      const authenticated = await checkAuth();
      if (!authenticated) {
        goto("/login");
      }
    }
  });

  // Watch for auth changes
  $: if (!$isAuthenticated && !isLoginPage && typeof window !== "undefined") {
    goto("/login");
  }

  function handleLogout() {
    logout();
    toasts.info("Déconnexion réussie");
    goto("/login");
  }
</script>

{#if isLoginPage}
  <slot />
{:else}
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
          <a href="/registries" class="nav-link">Registres</a>
          <a href="/logs" class="nav-link">Logs</a>
        </div>

        <div class="nav-actions">
          {#if $isAuthenticated && $user}
            <span class="user-badge">{$user.username}</span>
            <button class="btn btn-secondary" on:click={handleLogout}>
              <svg
                class="btn-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
                <polyline points="16 17 21 12 16 7"></polyline>
                <line x1="21" y1="12" x2="9" y2="12"></line>
              </svg>
            </button>
          {/if}
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
{/if}

<Toast />

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

  .nav-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .user-badge {
    padding: var(--spacing-xs) var(--spacing-sm);
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius-sm);
    font-size: 0.875rem;
    color: var(--text-secondary);
  }
</style>
