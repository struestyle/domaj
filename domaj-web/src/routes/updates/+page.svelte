<script>
    import { onMount } from "svelte";
    import { getUpdates } from "$lib/api.js";

    let updates = [];
    let loading = true;
    let error = null;
    let filter = "all";

    $: filteredUpdates =
        filter === "all"
            ? updates
            : filter === "same_tag"
              ? updates.filter((u) => u.same_tag_update)
              : updates.filter((u) => u.latest_update);

    onMount(async () => {
        try {
            updates = await getUpdates();
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    });
</script>

<svelte:head>
    <title>Mises à jour - Domaj</title>
</svelte:head>

<div class="container">
    <header class="page-header">
        <div>
            <h1>Mises à jour</h1>
            <p class="text-muted">
                Conteneurs avec des mises à jour disponibles
            </p>
        </div>
    </header>

    {#if loading}
        <div class="updates-list">
            {#each [1, 2, 3] as _}
                <div class="card">
                    <div
                        class="skeleton"
                        style="height: 24px; width: 50%; margin-bottom: 10px;"
                    ></div>
                    <div
                        class="skeleton"
                        style="height: 16px; width: 70%;"
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
            <h3>Tout est à jour !</h3>
            <p class="text-muted">
                Aucune mise à jour disponible pour vos conteneurs.
            </p>
        </div>
    {:else}
        <!-- Filters -->
        <div class="filters mb-lg">
            <button
                class="filter-btn {filter === 'all' ? 'active' : ''}"
                on:click={() => (filter = "all")}
            >
                Tous ({updates.length})
            </button>
            <button
                class="filter-btn {filter === 'same_tag' ? 'active' : ''}"
                on:click={() => (filter = "same_tag")}
            >
                🔄 Même tag ({updates.filter((u) => u.same_tag_update).length})
            </button>
            <button
                class="filter-btn {filter === 'latest' ? 'active' : ''}"
                on:click={() => (filter = "latest")}
            >
                🆕 Nouvelle version ({updates.filter((u) => u.latest_update)
                    .length})
            </button>
        </div>

        <!-- Updates List -->
        <div class="updates-list">
            {#each filteredUpdates as update}
                <div class="update-card card">
                    <div class="update-main">
                        <div class="update-header">
                            <span class="server-badge"
                                >{update.server_name}</span
                            >
                            <span class="update-time text-xs text-muted">
                                {#if update.last_checked}
                                    Vérifié: {new Date(
                                        update.last_checked,
                                    ).toLocaleString("fr-FR")}
                                {/if}
                            </span>
                        </div>
                        <div class="update-name">{update.container_name}</div>
                        <code class="update-image">{update.image}</code>
                    </div>

                    <div class="update-badges">
                        {#if update.same_tag_update}
                            <div class="update-badge warning">
                                <span class="badge-icon">🔄</span>
                                <div class="badge-content">
                                    <div class="badge-title">
                                        Tag mis à jour
                                    </div>
                                    <div class="badge-desc">
                                        L'image a été modifiée sur le registre
                                    </div>
                                </div>
                            </div>
                        {/if}
                        {#if update.latest_update}
                            <div class="update-badge success">
                                <span class="badge-icon">🆕</span>
                                <div class="badge-content">
                                    <div class="badge-title">
                                        Nouvelle version
                                    </div>
                                    <div class="badge-desc">
                                        Tag "{update.latest_tag || "latest"}"
                                        disponible
                                    </div>
                                </div>
                            </div>
                        {/if}
                    </div>
                </div>
            {/each}
        </div>
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

    .filters {
        display: flex;
        gap: var(--spacing-sm);
        flex-wrap: wrap;
    }

    .filter-btn {
        padding: var(--spacing-sm) var(--spacing-md);
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius-sm);
        color: var(--text-secondary);
        font-size: 0.875rem;
        cursor: pointer;
        transition: all var(--transition-fast);
    }

    .filter-btn:hover {
        background: var(--bg-card-hover);
        color: var(--text-primary);
    }

    .filter-btn.active {
        background: var(--color-primary);
        border-color: var(--color-primary);
        color: white;
    }

    .updates-list {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-md);
    }

    .update-card {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: var(--spacing-lg);
    }

    .update-header {
        display: flex;
        align-items: center;
        gap: var(--spacing-md);
        margin-bottom: var(--spacing-sm);
    }

    .server-badge {
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.5px;
        color: var(--color-primary);
        background: rgba(102, 126, 234, 0.15);
        padding: 2px 8px;
        border-radius: 4px;
    }

    .update-name {
        font-size: 1.25rem;
        font-weight: 600;
        margin-bottom: var(--spacing-xs);
    }

    .update-image {
        font-size: 0.875rem;
        background: var(--bg-primary);
        padding: 4px 8px;
        border-radius: 4px;
    }

    .update-badges {
        display: flex;
        flex-direction: column;
        gap: var(--spacing-sm);
        min-width: 250px;
    }

    .update-badge {
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
        padding: var(--spacing-sm) var(--spacing-md);
        border-radius: var(--border-radius-sm);
    }

    .update-badge.warning {
        background: rgba(237, 137, 54, 0.15);
        border: 1px solid rgba(237, 137, 54, 0.3);
    }

    .update-badge.success {
        background: rgba(72, 187, 120, 0.15);
        border: 1px solid rgba(72, 187, 120, 0.3);
    }

    .badge-icon {
        font-size: 1.25rem;
    }

    .badge-title {
        font-weight: 600;
        font-size: 0.875rem;
    }

    .badge-desc {
        font-size: 0.75rem;
        color: var(--text-secondary);
    }

    .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
    }

    .empty-icon {
        font-size: 4rem;
        margin-bottom: var(--spacing-md);
    }

    @media (max-width: 768px) {
        .update-card {
            flex-direction: column;
        }

        .update-badges {
            width: 100%;
            min-width: 0;
        }
    }
</style>
