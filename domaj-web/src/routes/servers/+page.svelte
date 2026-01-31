<script>
    import { onMount } from "svelte";
    import {
        getServers,
        createServer,
        deleteServer,
        syncServer,
        updateServer,
    } from "$lib/api.js";

    let servers = [];
    let loading = true;
    let error = null;
    let showAddForm = false;
    let newServer = { name: "", endpoint: "" };
    let submitting = false;

    // Edit state
    let editingServer = null;
    let editForm = { name: "", endpoint: "" };

    onMount(async () => {
        await loadServers();
    });

    async function loadServers() {
        try {
            loading = true;
            servers = await getServers();
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
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
        } catch (e) {
            alert("Erreur: " + e.message);
        } finally {
            submitting = false;
        }
    }

    async function handleDeleteServer(id) {
        if (!confirm("Êtes-vous sûr de vouloir supprimer ce serveur ?")) return;

        try {
            await deleteServer(id);
            servers = servers.filter((s) => s.id !== id);
        } catch (e) {
            alert("Erreur: " + e.message);
        }
    }

    async function handleSyncServer(id) {
        try {
            await syncServer(id);
            alert("Synchronisation lancée");
            await loadServers();
        } catch (e) {
            alert("Erreur de synchronisation: " + e.message);
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
        } catch (e) {
            alert("Erreur: " + e.message);
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
        <div class="grid grid-3">
            {#each [1, 2, 3] as _}
                <div class="card">
                    <div
                        class="skeleton"
                        style="height: 24px; width: 70%; margin-bottom: 10px;"
                    ></div>
                    <div
                        class="skeleton"
                        style="height: 16px; width: 90%;"
                    ></div>
                </div>
            {/each}
        </div>
    {:else if error}
        <div class="card error-card">
            <p>⚠️ {error}</p>
        </div>
    {:else if servers.length === 0}
        <div class="empty-state card">
            <div class="empty-icon">📦</div>
            <h3>Aucun serveur configuré</h3>
            <p class="text-muted">
                Ajoutez votre premier serveur pour commencer.
            </p>
        </div>
    {:else}
        <div class="servers-list">
            {#each servers as server}
                <div class="server-card card">
                    <div class="server-main">
                        <div
                            class="server-status {server.last_seen
                                ? 'online'
                                : 'offline'}"
                        ></div>
                        <div class="server-info">
                            <a href="/servers/{server.id}" class="server-name"
                                >{server.name}</a
                            >
                            <div class="server-endpoint text-sm text-muted">
                                {server.endpoint}
                            </div>
                            {#if server.last_seen}
                                <div class="server-lastseen text-xs text-muted">
                                    Dernière synchronisation: {new Date(
                                        server.last_seen,
                                    ).toLocaleString("fr-FR")}
                                </div>
                            {/if}
                        </div>
                    </div>
                    <div class="server-actions">
                        <button
                            class="btn btn-secondary"
                            on:click={() => handleSyncServer(server.id)}
                        >
                            🔄 Sync
                        </button>
                        <button
                            class="btn btn-secondary"
                            on:click={() => startEdit(server)}
                        >
                            ✏️
                        </button>
                        <a
                            href="/servers/{server.id}"
                            class="btn btn-secondary"
                        >
                            👁️ Voir
                        </a>
                        <button
                            class="btn btn-danger"
                            on:click={() => handleDeleteServer(server.id)}
                        >
                            🗑️
                        </button>
                    </div>
                </div>
            {/each}
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

    .servers-list {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-md);
    }

    .server-card {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .server-main {
        display: flex;
        align-items: center;
        gap: var(--spacing-md);
    }

    .server-status {
        width: 12px;
        height: 12px;
        border-radius: 50%;
        flex-shrink: 0;
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
        color: var(--text-primary);
    }

    .server-name:hover {
        color: var(--color-primary);
    }

    .server-actions {
        display: flex;
        gap: var(--spacing-sm);
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

        .server-card {
            flex-direction: column;
            align-items: flex-start;
            gap: var(--spacing-md);
        }

        .server-actions {
            width: 100%;
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
