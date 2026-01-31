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

    let sortColumn = "container_name";
    let sortDirection = "asc";
    let copiedId = null;

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

    function parseImage(image) {
        const parts = image.split(":");
        const tag = parts.length > 1 ? parts[parts.length - 1] : "latest";
        const name = parts.slice(0, -1).join(":") || image;
        return { name, tag };
    }

    function sortBy(column) {
        if (sortColumn === column) {
            sortDirection = sortDirection === "asc" ? "desc" : "asc";
        } else {
            sortColumn = column;
            sortDirection = "asc";
        }
    }

    $: sortedUpdates = [...updates].sort((a, b) => {
        let aVal, bVal;

        if (sortColumn === "image_name") {
            aVal = parseImage(a.image).name;
            bVal = parseImage(b.image).name;
        } else if (sortColumn === "current_tag") {
            aVal = parseImage(a.image).tag;
            bVal = parseImage(b.image).tag;
        } else if (sortColumn === "update_type") {
            aVal = a.same_tag_update ? "patch" : "new";
            bVal = b.same_tag_update ? "patch" : "new";
        } else {
            aVal = a[sortColumn] || "";
            bVal = b[sortColumn] || "";
        }

        if (typeof aVal === "string") aVal = aVal.toLowerCase();
        if (typeof bVal === "string") bVal = bVal.toLowerCase();

        if (aVal < bVal) return sortDirection === "asc" ? -1 : 1;
        if (aVal > bVal) return sortDirection === "asc" ? 1 : -1;
        return 0;
    });

    async function copyToClipboard(text, id) {
        try {
            await navigator.clipboard.writeText(text);
            copiedId = id;
            setTimeout(() => {
                copiedId = null;
            }, 1000);
        } catch (err) {
            console.error("Failed to copy:", err);
        }
    }

    function formatDigest(digest) {
        if (!digest) return "N/A";
        // Show short version: sha256:abc123...
        return digest.length > 20 ? digest.substring(0, 20) + "..." : digest;
    }
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
        <div class="grid grid-3">
            {#each [1, 2, 3] as _}
                <div class="stat-card">
                    <div
                        class="skeleton"
                        style="height: 16px; width: 50%;"
                    ></div>
                    <div
                        class="skeleton"
                        style="height: 24px; width: 30%;"
                    ></div>
                </div>
            {/each}
        </div>
    {:else if error}
        <div class="card error-card">
            <h3>
                <svg
                    class="icon icon-warning"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path
                        d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
                    ></path>
                    <line x1="12" y1="9" x2="12" y2="13"></line>
                    <line x1="12" y1="17" x2="12.01" y2="17"></line>
                </svg>
                Erreur de connexion
            </h3>
            <p class="text-muted">{error}</p>
            <p class="text-sm text-muted mt-md">
                Vérifiez que le serveur Domaj est en cours d'exécution.
            </p>
        </div>
    {:else}
        <!-- Stats Cards -->
        <div class="grid grid-3 mb-lg">
            <div class="stat-card">
                <div class="stat-icon">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <rect x="1" y="3" width="15" height="13" rx="2" ry="2"
                        ></rect>
                        <path
                            d="M16 8h2a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2v-2"
                        ></path>
                    </svg>
                </div>
                <div class="stat-content">
                    <div class="stat-value">{servers.length}</div>
                    <div class="stat-label">Serveurs</div>
                </div>
            </div>

            <div class="stat-card">
                <div class="stat-icon">
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                        ></path>
                        <polyline points="3.27 6.96 12 12.01 20.73 6.96"
                        ></polyline>
                        <line x1="12" y1="22.08" x2="12" y2="12"></line>
                    </svg>
                </div>
                <div class="stat-content">
                    <div class="stat-value">{containers.length}</div>
                    <div class="stat-label">Conteneurs</div>
                </div>
            </div>

            <div class="stat-card update-card">
                <div class="stat-icon">
                    <svg
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
                </div>
                <div class="stat-content">
                    <div class="stat-value">{updates.length}</div>
                    <div class="stat-label">Mises à jour</div>
                </div>
            </div>
        </div>

        <!-- Updates Section -->
        {#if updates.length > 0}
            <section class="section">
                <div class="section-header">
                    <h2>
                        <svg
                            class="icon-inline"
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
                        Mises à jour disponibles
                    </h2>
                </div>

                <div class="table-container card">
                    <table class="table">
                        <thead>
                            <tr>
                                <th
                                    class="sortable"
                                    on:click={() => sortBy("server_name")}
                                >
                                    Serveur {sortColumn === "server_name"
                                        ? sortDirection === "asc"
                                            ? "↑"
                                            : "↓"
                                        : ""}
                                </th>
                                <th
                                    class="sortable"
                                    on:click={() => sortBy("container_name")}
                                >
                                    Conteneur {sortColumn === "container_name"
                                        ? sortDirection === "asc"
                                            ? "↑"
                                            : "↓"
                                        : ""}
                                </th>
                                <th
                                    class="sortable"
                                    on:click={() => sortBy("image_name")}
                                >
                                    Image {sortColumn === "image_name"
                                        ? sortDirection === "asc"
                                            ? "↑"
                                            : "↓"
                                        : ""}
                                </th>
                                <th
                                    class="sortable"
                                    on:click={() => sortBy("current_tag")}
                                >
                                    Tag actuel {sortColumn === "current_tag"
                                        ? sortDirection === "asc"
                                            ? "↑"
                                            : "↓"
                                        : ""}
                                </th>
                                <th
                                    class="sortable"
                                    on:click={() => sortBy("update_type")}
                                >
                                    Type {sortColumn === "update_type"
                                        ? sortDirection === "asc"
                                            ? "↑"
                                            : "↓"
                                        : ""}
                                </th>
                                <th>Tag disponible</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each sortedUpdates as update}
                                {@const img = parseImage(update.image)}
                                <tr>
                                    <td>{update.server_name}</td>
                                    <td class="font-mono"
                                        >{update.container_name}</td
                                    >
                                    <td class="font-mono text-muted"
                                        >{img.name}</td
                                    >
                                    <td>
                                        <span
                                            class="tag-button"
                                            class:clickable={update.image_digest}
                                            data-tooltip={update.image_digest ||
                                                "Digest non disponible"}
                                            on:click={() =>
                                                update.image_digest &&
                                                copyToClipboard(
                                                    update.image_digest,
                                                    update.container_name +
                                                        update.server_name,
                                                )}
                                        >
                                            {#if copiedId === update.container_name + update.server_name}
                                                <span class="copied-feedback"
                                                    >Copié !</span
                                                >
                                            {:else}
                                                <code>{img.tag}</code>
                                                {#if update.image_digest}
                                                    <svg
                                                        class="copy-icon"
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2"
                                                    >
                                                        <rect
                                                            x="9"
                                                            y="9"
                                                            width="13"
                                                            height="13"
                                                            rx="2"
                                                            ry="2"
                                                        ></rect>
                                                        <path
                                                            d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                                                        ></path>
                                                    </svg>
                                                {/if}
                                            {/if}
                                        </span>
                                    </td>
                                    <td>
                                        {#if update.same_tag_update}
                                            <span class="badge badge-warning">
                                                <svg
                                                    class="badge-icon"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                >
                                                    <polyline
                                                        points="23 4 23 10 17 10"
                                                    ></polyline>
                                                    <polyline
                                                        points="1 20 1 14 7 14"
                                                    ></polyline>
                                                    <path
                                                        d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
                                                    ></path>
                                                </svg>
                                                Patch
                                            </span>
                                        {/if}
                                        {#if update.latest_update}
                                            <span class="badge badge-info">
                                                <svg
                                                    class="badge-icon"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                >
                                                    <circle
                                                        cx="12"
                                                        cy="12"
                                                        r="10"
                                                    ></circle>
                                                    <line
                                                        x1="12"
                                                        y1="8"
                                                        x2="12"
                                                        y2="12"
                                                    ></line>
                                                    <line
                                                        x1="12"
                                                        y1="16"
                                                        x2="12.01"
                                                        y2="16"
                                                    ></line>
                                                </svg>
                                                Nouveau
                                            </span>
                                        {/if}
                                    </td>
                                    <td>
                                        {#if update.latest_update || update.same_tag_update}
                                            {@const availableDigest =
                                                update.latest_digest ||
                                                update.same_tag_digest}
                                            {@const availableTag =
                                                update.latest_update
                                                    ? update.latest_tag ||
                                                      "latest"
                                                    : update.image.split(
                                                          ":",
                                                      )[1] || "latest"}
                                            <span
                                                class="tag-button"
                                                class:clickable={availableDigest}
                                                data-tooltip={availableDigest ||
                                                    "Digest non disponible"}
                                                on:click={() =>
                                                    availableDigest &&
                                                    copyToClipboard(
                                                        availableDigest,
                                                        "available-" +
                                                            update.container_name +
                                                            update.server_name,
                                                    )}
                                            >
                                                {#if copiedId === "available-" + update.container_name + update.server_name}
                                                    <span
                                                        class="copied-feedback"
                                                        >Copié !</span
                                                    >
                                                {:else}
                                                    <code>{availableTag}</code>
                                                    {#if availableDigest}
                                                        <svg
                                                            class="copy-icon"
                                                            viewBox="0 0 24 24"
                                                            fill="none"
                                                            stroke="currentColor"
                                                            stroke-width="2"
                                                        >
                                                            <rect
                                                                x="9"
                                                                y="9"
                                                                width="13"
                                                                height="13"
                                                                rx="2"
                                                                ry="2"
                                                            ></rect>
                                                            <path
                                                                d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
                                                            ></path>
                                                        </svg>
                                                    {/if}
                                                {/if}
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
            </section>
        {:else}
            <section class="section">
                <div class="empty-state card">
                    <div class="empty-icon">
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                            <polyline points="22 4 12 14.01 9 11.01"></polyline>
                        </svg>
                    </div>
                    <h3>Aucune mise à jour disponible</h3>
                    <p class="text-muted">
                        Tous vos conteneurs sont à jour ! Lancez un scan depuis
                        la barre de navigation.
                    </p>
                </div>
            </section>
        {/if}
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
        padding: var(--spacing-sm) var(--spacing-md);
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
        width: 40px;
        height: 40px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .stat-icon svg {
        width: 32px;
        height: 32px;
        stroke: var(--color-primary);
    }

    .stat-icon .icon-success {
        stroke: var(--color-success);
    }

    .stat-icon .icon-danger {
        stroke: var(--color-danger);
    }

    .stat-content {
        display: flex;
        align-items: baseline;
        gap: var(--spacing-sm);
    }

    .stat-value {
        font-size: 1.5rem;
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

    .table-container {
        overflow: visible;
        padding: 0;
    }

    .table-container .table {
        margin: 0;
    }

    .sortable {
        cursor: pointer;
        user-select: none;
        transition: background-color var(--transition-fast);
    }

    .sortable:hover {
        background: var(--bg-card-hover);
    }

    code {
        font-size: 0.8rem;
        background: var(--bg-primary);
        padding: 2px 8px;
        border-radius: 4px;
        color: var(--color-primary);
    }

    .tag-button {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 2px 4px;
        border-radius: 4px;
        transition: background-color var(--transition-fast);
        position: relative;
    }

    .tag-button[data-tooltip]::after {
        content: attr(data-tooltip);
        position: absolute;
        bottom: 100%;
        left: 50%;
        transform: translateX(-50%);
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 6px 10px;
        border-radius: 4px;
        font-size: 0.75rem;
        font-family: var(--font-mono);
        white-space: nowrap;
        opacity: 0;
        visibility: hidden;
        transition:
            opacity 0.1s ease,
            visibility 0.1s ease;
        pointer-events: none;
        z-index: 100;
        margin-bottom: 4px;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    .tag-button[data-tooltip]:hover::after {
        opacity: 1;
        visibility: visible;
    }

    .tag-button.clickable {
        cursor: pointer;
    }

    .tag-button.clickable:hover {
        background: var(--bg-card-hover);
    }

    .copy-icon {
        width: 14px;
        height: 14px;
        opacity: 0.3;
        transition: opacity var(--transition-fast);
        flex-shrink: 0;
    }

    .tag-button.clickable:hover .copy-icon {
        opacity: 1;
    }

    .copied-feedback {
        color: var(--color-primary);
        font-size: 0.8rem;
        font-weight: 500;
    }

    .icon {
        width: 20px;
        height: 20px;
        vertical-align: middle;
        margin-right: 0.5rem;
    }

    .icon-warning {
        stroke: var(--color-warning);
    }

    .icon-inline {
        width: 20px;
        height: 20px;
        vertical-align: middle;
        margin-right: 0.5rem;
        stroke: currentColor;
    }

    .badge-icon {
        width: 12px;
        height: 12px;
        margin-right: 4px;
        vertical-align: middle;
    }

    .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
    }

    .empty-icon {
        margin-bottom: var(--spacing-md);
        display: flex;
        justify-content: center;
    }

    .empty-icon svg {
        width: 64px;
        height: 64px;
        stroke: var(--color-success);
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
