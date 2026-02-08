<script>
    import { onMount } from "svelte";
    import { getRegistries } from "$lib/api.js";

    let registries = [];
    let loading = true;
    let error = null;

    onMount(async () => {
        try {
            registries = await getRegistries();
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    });

    async function refreshRegistries() {
        loading = true;
        error = null;
        try {
            registries = await getRegistries();
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    }

    function getStatusLabel(status) {
        switch (status) {
            case "accessible":
                return "Accessible";
            case "auth_failed":
                return "Authentification échouée";
            case "unreachable":
                return "Injoignable";
            default:
                return status;
        }
    }

    function getStatusClass(status) {
        switch (status) {
            case "accessible":
                return "badge-success";
            case "auth_failed":
                return "badge-warning";
            case "unreachable":
                return "badge-danger";
            default:
                return "";
        }
    }
</script>

<svelte:head>
    <title>Registres - Domaj</title>
</svelte:head>

<div class="container">
    <header class="page-header">
        <div>
            <h1>Registres de conteneurs</h1>
            <p class="text-muted">
                Registres détectés à partir de vos conteneurs
            </p>
        </div>
        <button
            class="btn btn-secondary"
            on:click={refreshRegistries}
            disabled={loading}
        >
            <svg
                class="btn-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <polyline points="23 4 23 10 17 10"></polyline>
                <polyline points="1 20 1 14 7 14"></polyline>
                <path
                    d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
                ></path>
            </svg>
            Rafraîchir
        </button>
    </header>

    {#if loading}
        <div class="skeleton" style="height: 200px;"></div>
    {:else if error}
        <div class="card error-card">
            <p>⚠️ {error}</p>
        </div>
    {:else if registries.length === 0}
        <div class="empty-state card">
            <div class="empty-icon">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    width="64"
                    height="64"
                >
                    <path
                        d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                    ></path>
                    <polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline>
                    <line x1="12" y1="22.08" x2="12" y2="12"></line>
                </svg>
            </div>
            <p class="text-muted">
                Aucun registre détecté. Synchronisez vos serveurs pour détecter
                les registres.
            </p>
        </div>
    {:else}
        <div class="table-wrapper card">
            <table class="table">
                <thead>
                    <tr>
                        <th>Statut</th>
                        <th>Registre</th>
                        <th>Identifiants</th>
                        <th>Conteneurs</th>
                        <th>Détails</th>
                    </tr>
                </thead>
                <tbody>
                    {#each registries as registry}
                        <tr>
                            <td>
                                <span
                                    class="badge {getStatusClass(
                                        registry.status,
                                    )}"
                                >
                                    {getStatusLabel(registry.status)}
                                </span>
                            </td>
                            <td>
                                <code class="registry-host"
                                    >{registry.host}</code
                                >
                            </td>
                            <td>
                                {#if registry.has_credentials}
                                    <span class="credential-badge configured">
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            width="14"
                                            height="14"
                                        >
                                            <rect
                                                x="3"
                                                y="11"
                                                width="18"
                                                height="11"
                                                rx="2"
                                                ry="2"
                                            ></rect>
                                            <path d="M7 11V7a5 5 0 0 1 10 0v4"
                                            ></path>
                                        </svg>
                                        Configurés
                                    </span>
                                {:else}
                                    <span
                                        class="credential-badge not-configured"
                                    >
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            width="14"
                                            height="14"
                                        >
                                            <rect
                                                x="3"
                                                y="11"
                                                width="18"
                                                height="11"
                                                rx="2"
                                                ry="2"
                                            ></rect>
                                            <path d="M7 11V7a5 5 0 0 1 10 0v4"
                                            ></path>
                                        </svg>
                                        Non configurés
                                    </span>
                                {/if}
                            </td>
                            <td>
                                <span class="container-count"
                                    >{registry.container_count}</span
                                >
                            </td>
                            <td>
                                {#if registry.error}
                                    <span
                                        class="error-detail text-muted"
                                        title={registry.error}
                                    >
                                        {registry.error.length > 60
                                            ? registry.error.slice(0, 60) +
                                              "..."
                                            : registry.error}
                                    </span>
                                {:else}
                                    <span class="text-muted">—</span>
                                {/if}
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>

<style>
    .page-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        margin-bottom: var(--spacing-xl);
    }

    .page-header h1 {
        font-size: 2rem;
        font-weight: 700;
        margin-bottom: var(--spacing-xs);
    }

    .btn-icon {
        width: 16px;
        height: 16px;
        margin-right: 6px;
        vertical-align: middle;
    }

    .table-wrapper {
        overflow: hidden;
    }

    .registry-host {
        font-size: 0.875rem;
        background: var(--bg-primary);
        padding: 4px 8px;
        border-radius: 4px;
        font-weight: 500;
    }

    .credential-badge {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-size: 0.8125rem;
        padding: 4px 10px;
        border-radius: 4px;
    }

    .credential-badge.configured {
        background: rgba(46, 204, 113, 0.1);
        color: var(--color-success);
    }

    .credential-badge.not-configured {
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-secondary);
    }

    .container-count {
        font-weight: 600;
        font-size: 0.9375rem;
    }

    .error-detail {
        font-size: 0.75rem;
        max-width: 300px;
        display: inline-block;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
    }

    .empty-icon {
        margin-bottom: var(--spacing-md);
        color: var(--text-muted);
    }

    .error-card {
        padding: var(--spacing-lg);
        color: var(--color-danger);
    }

    @media (max-width: 768px) {
        .page-header {
            flex-direction: column;
            gap: var(--spacing-md);
        }
    }
</style>
