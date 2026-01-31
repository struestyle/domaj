<script>
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import { getServer, getServerContainers, syncServer } from "$lib/api.js";

    let server = null;
    let containers = [];
    let loading = true;
    let error = null;
    let syncing = false;

    $: serverId = $page.params.id;

    onMount(async () => {
        await loadServer();
    });

    async function loadServer() {
        try {
            loading = true;
            [server, containers] = await Promise.all([
                getServer(serverId),
                getServerContainers(serverId),
            ]);
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    }

    async function handleSync() {
        try {
            syncing = true;
            containers = await syncServer(serverId);
            server = await getServer(serverId);
        } catch (e) {
            alert("Erreur: " + e.message);
        } finally {
            syncing = false;
        }
    }
</script>

<svelte:head>
    <title>{server?.name || "Serveur"} - Domaj</title>
</svelte:head>

<div class="container">
    {#if loading}
        <div
            class="skeleton"
            style="height: 40px; width: 200px; margin-bottom: 20px;"
        ></div>
        <div class="skeleton" style="height: 200px;"></div>
    {:else if error}
        <div class="card error-card">
            <p>⚠️ {error}</p>
            <a href="/servers" class="btn btn-secondary mt-md"
                >← Retour aux serveurs</a
            >
        </div>
    {:else}
        <header class="page-header">
            <div class="header-main">
                <a href="/servers" class="back-link text-muted">← Serveurs</a>
                <h1>
                    <span
                        class="server-status {server.last_seen
                            ? 'online'
                            : 'offline'}"
                    ></span>
                    {server.name}
                </h1>
                <p class="server-endpoint text-muted">{server.endpoint}</p>
            </div>
            <div class="header-actions">
                <button
                    class="btn btn-primary"
                    on:click={handleSync}
                    disabled={syncing}
                >
                    {syncing ? "🔄 Synchronisation..." : "🔄 Synchroniser"}
                </button>
            </div>
        </header>

        <section class="section">
            <h2>🐳 Conteneurs ({containers.length})</h2>

            {#if containers.length === 0}
                <div class="empty-state card">
                    <p class="text-muted">
                        Aucun conteneur trouvé. Lancez une synchronisation.
                    </p>
                </div>
            {:else}
                <div class="table-wrapper">
                    <table class="table">
                        <thead>
                            <tr>
                                <th>Nom</th>
                                <th>Image</th>
                                <th>Statut</th>
                                <th>ID</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each containers as container}
                                <tr>
                                    <td>
                                        <div class="container-name">
                                            {container.name}
                                        </div>
                                    </td>
                                    <td>
                                        <code class="image-tag"
                                            >{container.image}</code
                                        >
                                    </td>
                                    <td>
                                        <span
                                            class="badge {container.status.includes(
                                                'Up',
                                            )
                                                ? 'badge-success'
                                                : 'badge-warning'}"
                                        >
                                            {container.status.includes("Up")
                                                ? "🟢"
                                                : "🟡"}
                                            {container.status.split(" ")[0]}
                                        </span>
                                    </td>
                                    <td>
                                        <code class="text-muted text-sm"
                                            >{container.container_id.slice(
                                                0,
                                                12,
                                            )}</code
                                        >
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        </section>

        <section class="section">
            <h2>📋 Informations</h2>
            <div class="info-grid card">
                <div class="info-item">
                    <div class="info-label">Dernière synchronisation</div>
                    <div class="info-value">
                        {#if server.last_seen}
                            {new Date(server.last_seen).toLocaleString("fr-FR")}
                        {:else}
                            Jamais
                        {/if}
                    </div>
                </div>
                <div class="info-item">
                    <div class="info-label">Créé le</div>
                    <div class="info-value">
                        {new Date(server.created_at).toLocaleString("fr-FR")}
                    </div>
                </div>
                <div class="info-item">
                    <div class="info-label">Clé API</div>
                    <div class="info-value">
                        <code class="api-key"
                            >{server.api_key.slice(
                                0,
                                8,
                            )}...{server.api_key.slice(-4)}</code
                        >
                    </div>
                </div>
            </div>
        </section>
    {/if}
</div>

<style>
    .page-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        margin-bottom: var(--spacing-xl);
    }

    .back-link {
        font-size: 0.875rem;
        display: block;
        margin-bottom: var(--spacing-sm);
    }

    .page-header h1 {
        font-size: 2rem;
        font-weight: 700;
        display: flex;
        align-items: center;
        gap: var(--spacing-md);
    }

    .server-status {
        width: 14px;
        height: 14px;
        border-radius: 50%;
        display: inline-block;
    }

    .server-status.online {
        background: var(--color-success);
        box-shadow: 0 0 8px var(--color-success);
    }

    .server-status.offline {
        background: var(--text-muted);
    }

    .section {
        margin-top: var(--spacing-xl);
    }

    .section h2 {
        font-size: 1.25rem;
        margin-bottom: var(--spacing-lg);
    }

    .table-wrapper {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        overflow: hidden;
    }

    .container-name {
        font-weight: 500;
    }

    .image-tag {
        font-size: 0.75rem;
        background: var(--bg-primary);
        padding: 2px 6px;
        border-radius: 4px;
    }

    .info-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: var(--spacing-lg);
    }

    .info-label {
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: var(--text-secondary);
        margin-bottom: var(--spacing-xs);
    }

    .info-value {
        font-weight: 500;
    }

    .api-key {
        font-size: 0.875rem;
        background: var(--bg-primary);
        padding: 4px 8px;
        border-radius: 4px;
    }

    .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
    }

    @media (max-width: 768px) {
        .page-header {
            flex-direction: column;
            gap: var(--spacing-md);
        }

        .info-grid {
            grid-template-columns: 1fr;
        }
    }
</style>
