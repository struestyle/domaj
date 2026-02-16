<script>
    import { onMount } from "svelte";
    import { page } from "$app/stores";
    import { getServer, getServerContainers, syncServer } from "$lib/api.js";
    import { toasts } from "$lib/stores/toast.js";

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
            toasts.error("Erreur: " + e.message);
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
            <p>
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    width="16"
                    height="16"
                    style="vertical-align: middle; margin-right: 4px;"
                    ><path
                        d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
                    ></path><line x1="12" y1="9" x2="12" y2="13"></line><line
                        x1="12"
                        y1="17"
                        x2="12.01"
                        y2="17"
                    ></line></svg
                >
                {error}
            </p>
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
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        width="16"
                        height="16"
                        style="vertical-align: middle; margin-right: 6px;"
                        class:spinning={syncing}
                        ><polyline points="23 4 23 10 17 10"
                        ></polyline><polyline points="1 20 1 14 7 14"
                        ></polyline><path
                            d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
                        ></path></svg
                    >
                    {syncing ? "Synchronisation..." : "Synchroniser"}
                </button>
            </div>
        </header>

        <section class="section">
            <h2>
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    width="20"
                    height="20"
                    style="vertical-align: middle; margin-right: 6px;"
                    ><rect x="2" y="7" width="20" height="14" rx="2" ry="2"
                    ></rect><polyline points="17 2 12 7 7 2"></polyline></svg
                >Conteneurs ({containers.length})
            </h2>

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
                                            <span
                                                class="status-dot {container.status.includes(
                                                    'Up',
                                                )
                                                    ? 'dot-up'
                                                    : 'dot-down'}"
                                            ></span>
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
            <h2>
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    width="20"
                    height="20"
                    style="vertical-align: middle; margin-right: 6px;"
                    ><path
                        d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                    ></path><polyline points="14 2 14 8 20 8"></polyline><line
                        x1="16"
                        y1="13"
                        x2="8"
                        y2="13"
                    ></line><line x1="16" y1="17" x2="8" y2="17"
                    ></line><polyline points="10 9 9 9 8 9"></polyline></svg
                >Informations
            </h2>
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
        grid-template-columns: repeat(2, 1fr);
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

    .status-dot {
        display: inline-block;
        width: 8px;
        height: 8px;
        border-radius: 50%;
        margin-right: 6px;
        vertical-align: middle;
    }

    .dot-up {
        background: var(--color-success);
        box-shadow: 0 0 6px var(--color-success);
    }

    .dot-down {
        background: #eab308;
        box-shadow: 0 0 6px #eab308;
    }

    .spinning {
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
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
