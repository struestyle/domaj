<script>
    import { onMount } from "svelte";
    import {
        getServers,
        getContainers,
        getUpdates,
        getStatus,
    } from "$lib/api.js";

    let servers = [];
    let containers = [];
    let updates = [];
    let status = null;
    let loading = true;
    let error = null;

    onMount(async () => {
        try {
            [servers, containers, updates, status] = await Promise.all([
                getServers(),
                getContainers(),
                getUpdates(),
                getStatus(),
            ]);
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    });
</script>

<svelte:head>
    <title>Dashboard - Domaj</title>
</svelte:head>

<div class="container">
    <header class="page-header">
        <h1>Dashboard</h1>
        <p class="text-muted">Vue d'ensemble de vos instances Docker</p>
    </header>

    {#if loading}
        <div class="grid grid-4">
            {#each [1, 2, 3, 4] as _}
                <div class="card">
                    <div
                        class="skeleton"
                        style="height: 20px; width: 60%; margin-bottom: 10px;"
                    ></div>
                    <div
                        class="skeleton"
                        style="height: 40px; width: 40%;"
                    ></div>
                </div>
            {/each}
        </div>
    {:else if error}
        <div class="card error-card">
            <h3>⚠️ Erreur de connexion</h3>
            <p class="text-muted">{error}</p>
            <p class="text-sm text-muted mt-md">
                Vérifiez que le serveur Domaj est en cours d'exécution.
            </p>
        </div>
    {:else}
        <!-- Stats Cards -->
        <div class="grid grid-4 mb-lg">
            <div class="stat-card">
                <div class="stat-icon">📦</div>
                <div class="stat-content">
                    <div class="stat-value">{servers.length}</div>
                    <div class="stat-label">Serveurs</div>
                </div>
            </div>

            <div class="stat-card">
                <div class="stat-icon">🐳</div>
                <div class="stat-content">
                    <div class="stat-value">{containers.length}</div>
                    <div class="stat-label">Conteneurs</div>
                </div>
            </div>

            <div class="stat-card update-card">
                <div class="stat-icon">🔄</div>
                <div class="stat-content">
                    <div class="stat-value">{updates.length}</div>
                    <div class="stat-label">Mises à jour</div>
                </div>
            </div>

            <div class="stat-card">
                <div class="stat-icon">
                    {status?.status === "ok" ? "✅" : "❌"}
                </div>
                <div class="stat-content">
                    <div class="stat-value">
                        {status?.status === "ok" ? "En ligne" : "Hors ligne"}
                    </div>
                    <div class="stat-label">Statut API</div>
                </div>
            </div>
        </div>

        <!-- Updates Section -->
        {#if updates.length > 0}
            <section class="section">
                <div class="section-header">
                    <h2>🔄 Mises à jour disponibles</h2>
                    <a href="/updates" class="btn btn-secondary">Voir tout</a>
                </div>

                <div class="updates-list">
                    {#each updates.slice(0, 5) as update}
                        <div class="update-item card">
                            <div class="update-info">
                                <div class="update-server text-xs text-muted">
                                    {update.server_name}
                                </div>
                                <div class="update-name">
                                    {update.container_name}
                                </div>
                                <code class="update-image">{update.image}</code>
                            </div>
                            <div class="update-badges">
                                {#if update.same_tag_update}
                                    <span class="badge badge-warning"
                                        >🔄 Tag mis à jour</span
                                    >
                                {/if}
                                {#if update.latest_update}
                                    <span class="badge badge-success"
                                        >🆕 {update.latest_tag ||
                                            "latest"}</span
                                    >
                                {/if}
                            </div>
                        </div>
                    {/each}
                </div>
            </section>
        {/if}

        <!-- Servers Section -->
        <section class="section">
            <div class="section-header">
                <h2>📦 Serveurs</h2>
                <a href="/servers" class="btn btn-secondary">Gérer</a>
            </div>

            {#if servers.length === 0}
                <div class="empty-state card">
                    <div class="empty-icon">📦</div>
                    <h3>Aucun serveur configuré</h3>
                    <p class="text-muted">
                        Ajoutez votre premier serveur pour commencer à
                        surveiller vos conteneurs.
                    </p>
                    <a href="/servers" class="btn btn-primary mt-md"
                        >Ajouter un serveur</a
                    >
                </div>
            {:else}
                <div class="grid grid-3">
                    {#each servers as server}
                        <a href="/servers/{server.id}" class="server-card card">
                            <div
                                class="server-status {server.last_seen
                                    ? 'online'
                                    : 'offline'}"
                            ></div>
                            <div class="server-name">{server.name}</div>
                            <div
                                class="server-endpoint text-sm text-muted truncate"
                            >
                                {server.endpoint}
                            </div>
                        </a>
                    {/each}
                </div>
            {/if}
        </section>
    {/if}
</div>

<style>
    .page-header {
        margin-bottom: var(--spacing-xl);
    }

    .page-header h1 {
        font-size: 2rem;
        font-weight: 700;
        margin-bottom: var(--spacing-xs);
    }

    .stat-card {
        display: flex;
        align-items: center;
        gap: var(--spacing-md);
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        padding: var(--spacing-lg);
    }

    .stat-card.update-card {
        border-color: var(--color-warning);
        background: linear-gradient(
            135deg,
            rgba(237, 137, 54, 0.1) 0%,
            transparent 100%
        );
    }

    .stat-icon {
        font-size: 2rem;
    }

    .stat-value {
        font-size: 1.75rem;
        font-weight: 700;
    }

    .stat-label {
        color: var(--text-secondary);
        font-size: 0.875rem;
    }

    .section {
        margin-top: var(--spacing-xl);
    }

    .section-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: var(--spacing-lg);
    }

    .section-header h2 {
        font-size: 1.25rem;
        font-weight: 600;
    }

    .updates-list {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-md);
    }

    .update-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .update-name {
        font-weight: 600;
        margin: var(--spacing-xs) 0;
    }

    .update-image {
        font-size: 0.75rem;
        background: var(--bg-primary);
        padding: 2px 6px;
        border-radius: 4px;
    }

    .update-badges {
        display: flex;
        gap: var(--spacing-sm);
    }

    .server-card {
        position: relative;
        display: block;
    }

    .server-status {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        position: absolute;
        top: var(--spacing-md);
        right: var(--spacing-md);
    }

    .server-status.online {
        background: var(--color-success);
        box-shadow: 0 0 8px var(--color-success);
    }

    .server-status.offline {
        background: var(--text-muted);
    }

    .server-name {
        font-weight: 600;
        font-size: 1.125rem;
        margin-bottom: var(--spacing-xs);
    }

    .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
    }

    .empty-icon {
        font-size: 3rem;
        margin-bottom: var(--spacing-md);
    }

    .error-card {
        border-color: var(--color-danger);
        background: linear-gradient(
            135deg,
            rgba(245, 101, 101, 0.1) 0%,
            transparent 100%
        );
    }
</style>
