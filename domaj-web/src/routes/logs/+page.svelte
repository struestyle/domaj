<script>
    import { onMount, onDestroy } from "svelte";
    import { getUpdateJobs } from "$lib/api.js";
    import { websocketStore } from "$lib/stores/websocket.js";

    let jobs = [];
    let loading = true;
    let error = null;

    $: activeJobs = jobs.filter(
        (j) => j.status === "running" || j.status === "pending",
    );
    $: completedJobs = jobs.filter(
        (j) => j.status === "success" || j.status === "failed",
    );

    async function loadJobs() {
        try {
            jobs = await getUpdateJobs();
        } catch (e) {
            error = e.message;
        }
    }

    const unsubEvent = websocketStore.lastEvent.subscribe(async (event) => {
        if (!event) return;
        if (
            event.type === "job_started" ||
            event.type === "job_completed" ||
            event.type === "job_failed"
        ) {
            await loadJobs();
        }
    });

    onMount(async () => {
        websocketStore.connect();
        await loadJobs();
        loading = false;
    });

    onDestroy(() => {
        unsubEvent();
        websocketStore.disconnect();
    });

    function statusBadgeClass(status) {
        switch (status) {
            case "success":
                return "badge-success";
            case "failed":
                return "badge-danger";
            case "running":
                return "badge-info";
            case "pending":
                return "badge-warning";
            default:
                return "";
        }
    }

    function statusLabel(status) {
        switch (status) {
            case "success":
                return "Succès";
            case "failed":
                return "Échec";
            case "running":
                return "En cours";
            case "pending":
                return "En attente";
            default:
                return status;
        }
    }

    function formatDate(dateStr) {
        if (!dateStr) return "—";
        return new Date(dateStr).toLocaleString("fr-FR");
    }

    function duration(startedAt, completedAt) {
        if (!startedAt || !completedAt) return "—";
        const ms = new Date(completedAt) - new Date(startedAt);
        if (ms < 1000) return `${ms}ms`;
        return `${(ms / 1000).toFixed(1)}s`;
    }
</script>

<svelte:head>
    <title>Logs - Domaj</title>
</svelte:head>

<div class="container">
    <header class="page-header">
        <h1>Logs de mise à jour</h1>
        <p class="text-muted">
            Suivi en temps réel et historique des mises à jour
        </p>
    </header>

    {#if loading}
        <div class="card loading-card">
            <span class="spinner"></span> Chargement...
        </div>
    {:else if error}
        <div class="card error-card">
            <p>Erreur : {error}</p>
        </div>
    {:else}
        <!-- Active Jobs -->
        {#if activeJobs.length > 0}
            <section class="section">
                <div class="section-header">
                    <h2 class="section-title">
                        <svg
                            class="section-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <polyline points="1 4 1 10 7 10"></polyline>
                            <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
                        </svg>
                        File d'attente
                        <span class="badge badge-info">{activeJobs.length}</span
                        >
                    </h2>
                </div>
                <div class="jobs-grid">
                    {#each activeJobs as job (job.id)}
                        <div class="job-card active-job">
                            <div class="job-card-header">
                                <span class="spinner-sm"></span>
                                <strong>{job.container_name}</strong>
                                <span class="text-muted"
                                    >sur {job.server_name}</span
                                >
                            </div>
                            <div class="job-card-details">
                                <span class="monospace text-sm"
                                    >{job.image}</span
                                >
                                {#if job.target_tag}
                                    <span class="badge badge-info"
                                        >{job.target_tag}</span
                                    >
                                {/if}
                            </div>
                            <div class="job-card-footer text-muted text-sm">
                                Démarré {formatDate(job.started_at)}
                            </div>
                        </div>
                    {/each}
                </div>
            </section>
        {/if}

        <!-- History -->
        <section class="section">
            <div class="section-header">
                <h2 class="section-title">
                    <svg
                        class="section-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                        ></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                        <line x1="16" y1="13" x2="8" y2="13"></line>
                        <line x1="16" y1="17" x2="8" y2="17"></line>
                    </svg>
                    Historique
                </h2>
            </div>

            {#if completedJobs.length === 0 && activeJobs.length === 0}
                <div class="card empty-state">
                    <div class="empty-icon">
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path
                                d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                            ></path>
                            <polyline points="14 2 14 8 20 8"></polyline>
                            <line x1="16" y1="13" x2="8" y2="13"></line>
                            <line x1="16" y1="17" x2="8" y2="17"></line>
                        </svg>
                    </div>
                    <h3>Aucun log disponible</h3>
                    <p class="text-muted">
                        Lancez une mise à jour pour voir l'historique ici.
                    </p>
                </div>
            {:else}
                <div class="table-container card">
                    <table class="table">
                        <thead>
                            <tr>
                                <th>Conteneur</th>
                                <th>Serveur</th>
                                <th>Image</th>
                                <th>Tag cible</th>
                                <th>Statut</th>
                                <th>Démarré</th>
                                <th>Durée</th>
                                <th>Erreur</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each completedJobs as job (job.id)}
                                <tr class:job-failed={job.status === "failed"}>
                                    <td
                                        ><strong>{job.container_name}</strong
                                        ></td
                                    >
                                    <td>{job.server_name}</td>
                                    <td class="monospace image-cell"
                                        >{job.image}</td
                                    >
                                    <td>{job.target_tag || "—"}</td>
                                    <td>
                                        <span
                                            class="badge {statusBadgeClass(
                                                job.status,
                                            )}"
                                        >
                                            {statusLabel(job.status)}
                                        </span>
                                    </td>
                                    <td class="date-cell"
                                        >{formatDate(job.started_at)}</td
                                    >
                                    <td
                                        >{duration(
                                            job.started_at,
                                            job.completed_at,
                                        )}</td
                                    >
                                    <td class="error-cell">
                                        {#if job.error_message}
                                            <span
                                                class="error-text"
                                                title={job.error_message}
                                            >
                                                {job.error_message.length > 60
                                                    ? job.error_message.substring(
                                                          0,
                                                          60,
                                                      ) + "..."
                                                    : job.error_message}
                                            </span>
                                        {:else}
                                            —
                                        {/if}
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
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

    .section {
        margin-bottom: var(--spacing-xl);
    }

    .section-header {
        margin-bottom: var(--spacing-md);
    }

    .section-title {
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
        font-size: 1.25rem;
        font-weight: 600;
    }

    .section-icon {
        width: 22px;
        height: 22px;
        flex-shrink: 0;
    }

    .loading-card {
        padding: var(--spacing-lg);
        text-align: center;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: var(--spacing-sm);
    }

    /* Active jobs grid */
    .jobs-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
        gap: var(--spacing-md);
    }

    .job-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        padding: var(--spacing-md);
        display: flex;
        flex-direction: column;
        gap: var(--spacing-sm);
    }

    .active-job {
        border-color: var(--color-info, #3b82f6);
        background: linear-gradient(
            135deg,
            rgba(59, 130, 246, 0.08) 0%,
            transparent 100%
        );
        animation: pulse-border 2s ease-in-out infinite;
    }

    @keyframes pulse-border {
        0%,
        100% {
            border-color: var(--color-info, #3b82f6);
        }
        50% {
            border-color: rgba(59, 130, 246, 0.3);
        }
    }

    .job-card-header {
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
        font-size: 0.95rem;
    }

    .job-card-details {
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
        flex-wrap: wrap;
    }

    .job-card-footer {
        margin-top: auto;
    }

    /* Table */
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
        stroke: var(--text-secondary);
    }

    .image-cell {
        max-width: 250px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .date-cell {
        white-space: nowrap;
        font-size: 0.85rem;
    }

    .error-cell {
        max-width: 300px;
    }

    .error-text {
        color: var(--color-error, #ef4444);
        font-size: 0.8rem;
    }

    .error-card {
        border-color: var(--color-danger);
        padding: var(--spacing-lg);
        background: linear-gradient(
            135deg,
            rgba(245, 101, 101, 0.1) 0%,
            transparent 100%
        );
    }

    .job-failed {
        background: rgba(239, 68, 68, 0.05);
    }

    .text-sm {
        font-size: 0.85rem;
    }

    .spinner {
        display: inline-block;
        width: 14px;
        height: 14px;
        border: 2px solid currentColor;
        border-top-color: transparent;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
        vertical-align: middle;
    }

    .spinner-sm {
        display: inline-block;
        width: 12px;
        height: 12px;
        border: 2px solid var(--color-info, #3b82f6);
        border-top-color: transparent;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
        flex-shrink: 0;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
</style>
