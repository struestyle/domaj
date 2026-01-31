<script>
    import { onMount } from "svelte";
    import { getUpdates, triggerScan } from "$lib/api.js";

    let updates = [];
    let loading = true;
    let error = null;
    let scanning = false;

    onMount(async () => {
        await loadUpdates();
    });

    async function loadUpdates() {
        try {
            loading = true;
            updates = await getUpdates();
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    }

    async function handleScan() {
        try {
            scanning = true;
            await triggerScan();
            // Wait a bit for scan to complete
            setTimeout(async () => {
                await loadUpdates();
                scanning = false;
            }, 3000);
        } catch (e) {
            alert("Erreur: " + e.message);
            scanning = false;
        }
    }
</script>

<svelte:head>
    <title>Mises à jour - Domaj</title>
</svelte:head>

<div class="container">
    <header class="page-header">
        <div>
            <h1>Mises à jour disponibles</h1>
            <p class="text-muted">Conteneurs avec des mises à jour détectées</p>
        </div>
        <button
            class="btn btn-primary"
            on:click={handleScan}
            disabled={scanning}
        >
            {scanning ? "⏳ Scan en cours..." : "🔍 Lancer un scan"}
        </button>
    </header>

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
    {:else if updates.length === 0}
        <div class="empty-state card">
            <div class="empty-icon">✅</div>
            <h3>Aucune mise à jour disponible</h3>
            <p class="text-muted">
                Tous vos conteneurs sont à jour ! Lancez un scan pour vérifier.
            </p>
        </div>
    {:else}
        <div class="updates-summary">
            <div class="stat-card card">
                <span class="stat-number">{updates.length}</span>
                <span class="stat-label"
                    >Mise{updates.length > 1 ? "s" : ""} à jour</span
                >
            </div>
            <div class="stat-card card warning">
                <span class="stat-number"
                    >{updates.filter((u) => u.same_tag_update).length}</span
                >
                <span class="stat-label">Même tag (patch)</span>
            </div>
            <div class="stat-card card info">
                <span class="stat-number"
                    >{updates.filter((u) => u.latest_update).length}</span
                >
                <span class="stat-label">Nouvelle version</span>
            </div>
        </div>

        <div class="updates-list">
            {#each updates as update}
                <div class="update-card card">
                    <div class="update-main">
                        <div class="update-info">
                            <h3 class="update-name">{update.container_name}</h3>
                            <div
                                class="update-image font-mono text-sm text-muted"
                            >
                                {update.image}
                            </div>
                            <div class="update-server text-xs text-muted">
                                📦 {update.server_name}
                            </div>
                        </div>
                        <div class="update-badges">
                            {#if update.same_tag_update}
                                <span class="badge badge-warning">
                                    🔄 Patch disponible
                                </span>
                            {/if}
                            {#if update.latest_update}
                                <span class="badge badge-info">
                                    🆕 {update.latest_tag || "latest"}
                                </span>
                            {/if}
                        </div>
                    </div>
                    {#if update.last_checked}
                        <div class="update-checked text-xs text-muted">
                            Vérifié: {new Date(
                                update.last_checked,
                            ).toLocaleString("fr-FR")}
                        </div>
                    {/if}
                </div>
            {/each}
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

    .updates-summary {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: var(--spacing-md);
        margin-bottom: var(--spacing-xl);
    }

    .stat-card {
        text-align: center;
        padding: var(--spacing-lg);
    }

    .stat-number {
        display: block;
        font-size: 2.5rem;
        font-weight: 700;
        color: var(--color-primary);
    }

    .stat-card.warning .stat-number {
        color: var(--color-warning);
    }

    .stat-card.info .stat-number {
        color: var(--color-primary);
    }

    .stat-label {
        color: var(--text-secondary);
        font-size: 0.875rem;
    }

    .updates-list {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-md);
    }

    .update-card {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-sm);
    }

    .update-main {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
    }

    .update-name {
        font-size: 1.125rem;
        font-weight: 600;
        margin-bottom: var(--spacing-xs);
    }

    .update-badges {
        display: flex;
        gap: var(--spacing-sm);
        flex-shrink: 0;
        align-items: flex-start;
    }

    .update-badges .badge {
        cursor: pointer;
        width: fit-content;
        transition: opacity 0.15s ease;
    }

    .update-badges .badge:hover {
        opacity: 0.8;
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
        .updates-summary {
            grid-template-columns: 1fr;
        }

        .update-main {
            flex-direction: column;
            gap: var(--spacing-md);
        }
    }
</style>
