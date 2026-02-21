<script>
    import { onMount } from "svelte";
    import {
        getServers,
        createServer,
        deleteServer,
        syncServer,
        updateServer,
        checkServerHealth,
    } from "$lib/api.js";
    import { toasts } from "$lib/stores/toast.js";

    let servers = [];
    let loading = true;
    let error = null;
    let showAddForm = false;
    let newServer = { name: "", endpoint: "" };
    let submitting = false;

    // Edit state
    let editingServer = null;
    let editForm = { name: "", endpoint: "" };

    // Health check state: { [serverId]: { status: 'checking'|'ok'|'auth_error'|'unreachable', error: string|null, checking: boolean } }
    let healthStatus = {};

    onMount(async () => {
        await loadServers();
    });

    async function loadServers() {
        try {
            loading = true;
            servers = await getServers();
            // Auto-check health for all servers
            for (const server of servers) {
                checkHealth(server.id);
            }
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    }

    async function checkHealth(id) {
        healthStatus = {
            ...healthStatus,
            [id]: { status: "checking", error: null, checking: true },
        };
        try {
            const result = await checkServerHealth(id);
            if (result.reachable && result.authenticated) {
                healthStatus = {
                    ...healthStatus,
                    [id]: { status: "ok", error: null, checking: false },
                };
            } else if (result.reachable && !result.authenticated) {
                healthStatus = {
                    ...healthStatus,
                    [id]: {
                        status: "auth_error",
                        error: result.error,
                        checking: false,
                    },
                };
            } else {
                healthStatus = {
                    ...healthStatus,
                    [id]: {
                        status: "unreachable",
                        error: result.error,
                        checking: false,
                    },
                };
            }
        } catch (e) {
            healthStatus = {
                ...healthStatus,
                [id]: {
                    status: "unreachable",
                    error: e.message,
                    checking: false,
                },
            };
        }
    }

    async function handleAddServer() {
        if (!newServer.name || !newServer.endpoint) return;

        try {
            submitting = true;
            const created = await createServer(newServer);
            servers = [...servers, created];
            newServer = { name: "", endpoint: "" };
            showAddForm = false;
            checkHealth(created.id);
        } catch (e) {
            toasts.error("Erreur: " + e.message);
        } finally {
            submitting = false;
        }
    }

    async function handleDeleteServer(id) {
        if (!confirm("Êtes-vous sûr de vouloir supprimer ce serveur ?")) return;

        try {
            await deleteServer(id);
            servers = servers.filter((s) => s.id !== id);
            delete healthStatus[id];
            healthStatus = healthStatus;
        } catch (e) {
            toasts.error("Erreur: " + e.message);
        }
    }

    async function handleSyncServer(id) {
        try {
            await syncServer(id);
            toasts.success("Synchronisation lancée");
            await loadServers();
        } catch (e) {
            toasts.error("Erreur de synchronisation: " + e.message);
        }
    }

    function startEdit(server) {
        editingServer = server;
        editForm = { name: server.name, endpoint: server.endpoint };
    }

    function cancelEdit() {
        editingServer = null;
        editForm = { name: "", endpoint: "" };
    }

    async function handleUpdateServer() {
        if (!editForm.name || !editForm.endpoint) return;

        try {
            submitting = true;
            const updated = await updateServer(editingServer.id, editForm);
            servers = servers.map((s) => (s.id === updated.id ? updated : s));
            cancelEdit();
            checkHealth(updated.id);
        } catch (e) {
            toasts.error("Erreur: " + e.message);
        } finally {
            submitting = false;
        }
    }
</script>

<svelte:head>
    <title>Serveurs - Domaj</title>
</svelte:head>

<div class="container">
    <header class="page-header">
        <div>
            <h1>Serveurs</h1>
            <p class="text-muted">Gérez vos serveurs Docker surveillés</p>
        </div>
        <button
            class="btn btn-primary"
            on:click={() => (showAddForm = !showAddForm)}
        >
            {showAddForm ? "✕ Annuler" : "+ Ajouter un serveur"}
        </button>
    </header>

    {#if showAddForm}
        <div class="card add-form mb-lg">
            <h3>Nouveau serveur</h3>
            <form on:submit|preventDefault={handleAddServer}>
                <div class="form-grid">
                    <div class="form-group">
                        <label for="name">Nom du serveur</label>
                        <input
                            type="text"
                            id="name"
                            class="input"
                            placeholder="Mon serveur"
                            bind:value={newServer.name}
                            required
                        />
                    </div>
                    <div class="form-group">
                        <label for="endpoint">URL de l'agent</label>
                        <input
                            type="url"
                            id="endpoint"
                            class="input"
                            placeholder="http://192.168.1.100:3001"
                            bind:value={newServer.endpoint}
                            required
                        />
                    </div>
                </div>
                <div class="form-actions">
                    <button
                        type="submit"
                        class="btn btn-primary"
                        disabled={submitting}
                    >
                        {submitting ? "Ajout..." : "Ajouter le serveur"}
                    </button>
                </div>
            </form>
        </div>
    {/if}

    {#if loading}
        <div class="table-container card">
            <div class="skeleton" style="height: 200px;"></div>
        </div>
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
        </div>
    {:else if servers.length === 0}
        <div class="empty-state card">
            <div class="empty-icon">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    width="48"
                    height="48"
                    ><path
                        d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                    ></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"
                    ></polyline><line x1="12" y1="22.08" x2="12" y2="12"
                    ></line></svg
                >
            </div>
            <h3>Aucun serveur configuré</h3>
            <p class="text-muted">
                Ajoutez votre premier serveur pour commencer.
            </p>
        </div>
    {:else}
        <div class="table-container card">
            <table class="table">
                <thead>
                    <tr>
                        <th>Statut</th>
                        <th>Nom</th>
                        <th>Endpoint</th>
                        <th>Dernière synchronisation</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {#each servers as server}
                        {@const hs = healthStatus[server.id]}
                        <tr>
                            <td>
                                <span
                                    class="status-indicator {!hs ||
                                    hs.status === 'checking'
                                        ? 'status-checking'
                                        : hs.status === 'ok'
                                          ? 'status-online'
                                          : hs.status === 'auth_error'
                                            ? 'status-auth-error'
                                            : 'status-offline'}"
                                    title={hs?.error || ""}
                                >
                                    {#if hs?.checking}
                                        <span class="spinner"></span>
                                    {/if}
                                    {#if !hs || hs.status === "checking"}
                                        Vérification…
                                    {:else if hs.status === "ok"}
                                        Connecté
                                    {:else if hs.status === "auth_error"}
                                        Clé API invalide
                                    {:else}
                                        Injoignable
                                    {/if}
                                </span>
                            </td>
                            <td>
                                <a
                                    href="/servers/{server.id}"
                                    class="link-primary">{server.name}</a
                                >
                            </td>
                            <td>
                                <code class="endpoint-code"
                                    >{server.endpoint}</code
                                >
                            </td>
                            <td>
                                {#if server.last_seen}
                                    {new Date(server.last_seen).toLocaleString(
                                        "fr-FR",
                                    )}
                                {:else}
                                    <span class="text-muted">Jamais</span>
                                {/if}
                            </td>
                            <td>
                                <div class="action-buttons">
                                    <button
                                        class="btn btn-secondary"
                                        on:click={() => checkHealth(server.id)}
                                        title="Tester la connexion"
                                        disabled={healthStatus[server.id]
                                            ?.checking}
                                    >
                                        <svg
                                            class="btn-icon-only"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                        >
                                            <path
                                                d="M22 11.08V12a10 10 0 1 1-5.93-9.14"
                                            ></path>
                                            <polyline
                                                points="22 4 12 14.01 9 11.01"
                                            ></polyline>
                                        </svg>
                                    </button>
                                    <button
                                        class="btn btn-secondary"
                                        on:click={() =>
                                            handleSyncServer(server.id)}
                                        title="Synchroniser"
                                    >
                                        <svg
                                            class="btn-icon-only"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                        >
                                            <polyline points="23 4 23 10 17 10"
                                            ></polyline>
                                            <polyline points="1 20 1 14 7 14"
                                            ></polyline>
                                            <path
                                                d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
                                            ></path>
                                        </svg>
                                    </button>
                                    <button
                                        class="btn btn-secondary"
                                        on:click={() => startEdit(server)}
                                        title="Modifier"
                                    >
                                        <svg
                                            class="btn-icon-only"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                        >
                                            <path
                                                d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"
                                            ></path>
                                            <path
                                                d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"
                                            ></path>
                                        </svg>
                                    </button>
                                    <button
                                        class="btn btn-danger"
                                        on:click={() =>
                                            handleDeleteServer(server.id)}
                                        title="Supprimer"
                                    >
                                        <svg
                                            class="btn-icon-only"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                        >
                                            <polyline points="3 6 5 6 21 6"
                                            ></polyline>
                                            <path
                                                d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                            ></path>
                                        </svg>
                                    </button>
                                </div>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>

{#if editingServer}
    <div class="modal-overlay" on:click={cancelEdit}>
        <div class="modal" on:click|stopPropagation>
            <h3>Modifier le serveur</h3>
            <form on:submit|preventDefault={handleUpdateServer}>
                <div class="form-group">
                    <label for="edit-name">Nom du serveur</label>
                    <input
                        type="text"
                        id="edit-name"
                        class="input"
                        bind:value={editForm.name}
                        required
                    />
                </div>
                <div class="form-group">
                    <label for="edit-endpoint">URL de l'agent</label>
                    <input
                        type="url"
                        id="edit-endpoint"
                        class="input"
                        bind:value={editForm.endpoint}
                        required
                    />
                </div>
                <div class="modal-actions">
                    <button
                        type="button"
                        class="btn btn-secondary"
                        on:click={cancelEdit}
                    >
                        Annuler
                    </button>
                    <button
                        type="submit"
                        class="btn btn-primary"
                        disabled={submitting}
                    >
                        {submitting ? "Enregistrement..." : "Enregistrer"}
                    </button>
                </div>
            </form>
        </div>
    </div>
{/if}

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

    .add-form {
        padding: var(--spacing-lg);
    }

    .add-form h3 {
        margin-bottom: var(--spacing-lg);
    }

    .form-grid {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: var(--spacing-lg);
    }

    .form-group {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-sm);
    }

    .form-group label {
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-secondary);
    }

    .form-actions {
        margin-top: var(--spacing-lg);
    }

    .status-indicator {
        display: inline-block;
        padding: 4px 10px;
        border-radius: 12px;
        font-size: 0.75rem;
        font-weight: 500;
    }

    .status-online {
        background: rgba(16, 185, 129, 0.15);
        color: var(--color-success);
    }

    .status-auth-error {
        background: rgba(245, 158, 11, 0.15);
        color: #f59e0b;
    }

    .status-offline {
        background: rgba(239, 68, 68, 0.15);
        color: #ef4444;
    }

    .status-checking {
        background: rgba(156, 163, 175, 0.1);
        color: var(--text-muted);
    }

    .spinner {
        display: inline-block;
        width: 10px;
        height: 10px;
        border: 2px solid currentColor;
        border-right-color: transparent;
        border-radius: 50%;
        animation: spin 0.6s linear infinite;
        margin-right: 4px;
        vertical-align: middle;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .link-primary {
        color: var(--color-primary);
        font-weight: 500;
    }

    .link-primary:hover {
        text-decoration: underline;
    }

    .endpoint-code {
        font-family: monospace;
        font-size: 0.8rem;
        padding: 2px 6px;
        background: var(--bg-tertiary);
        border-radius: 4px;
        color: var(--text-secondary);
    }

    .action-buttons {
        white-space: nowrap;
    }

    .btn-icon-only {
        width: 16px;
        height: 16px;
    }

    .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
    }

    .empty-icon {
        font-size: 3rem;
        margin-bottom: var(--spacing-md);
    }

    @media (max-width: 768px) {
        .form-grid {
            grid-template-columns: 1fr;
        }
    }

    /* Modal styles */
    .modal-overlay {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.7);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .modal {
        background: var(--bg-secondary);
        border-radius: var(--border-radius-lg);
        padding: var(--spacing-xl);
        max-width: 500px;
        width: 90%;
        border: 1px solid var(--border-color);
    }

    .modal h3 {
        margin-bottom: var(--spacing-lg);
    }

    .modal .form-group {
        margin-bottom: var(--spacing-md);
    }

    .modal-actions {
        display: flex;
        gap: var(--spacing-md);
        justify-content: flex-end;
        margin-top: var(--spacing-lg);
    }
</style>
