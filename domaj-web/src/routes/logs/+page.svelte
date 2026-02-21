<script>
    import { onMount, onDestroy } from "svelte";
    import { getUpdateJobs, rollbackJob, getAuditLogs } from "$lib/api.js";
    import { websocketStore } from "$lib/stores/websocket.js";

    let jobs = [];
    let loading = true;
    let error = null;
    let rollingBack = null;

    // Audit logs state
    let auditLogs = [];
    let auditTotal = 0;
    let auditOffset = 0;
    const auditLimit = 30;
    let auditLoading = false;

    async function handleRollback(job) {
        if (rollingBack) return;
        rollingBack = job.id;
        try {
            await rollbackJob(job.id);
        } catch (e) {
            error = `Rollback failed: ${e.message}`;
        } finally {
            rollingBack = null;
        }
    }

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

    async function loadAuditLogs() {
        auditLoading = true;
        try {
            const data = await getAuditLogs(auditLimit, auditOffset);
            auditLogs = data.logs;
            auditTotal = data.total;
        } catch (e) {
            console.error("Failed to load audit logs:", e);
        } finally {
            auditLoading = false;
        }
    }

    async function auditPrev() {
        auditOffset = Math.max(0, auditOffset - auditLimit);
        await loadAuditLogs();
    }

    async function auditNext() {
        if (auditOffset + auditLimit < auditTotal) {
            auditOffset += auditLimit;
            await loadAuditLogs();
        }
    }

    function actionBadgeClass(action) {
        switch (action) {
            case "login":
                return "badge-login";
            case "scan":
                return "badge-scan";
            case "update":
                return "badge-update";
            case "rollback":
                return "badge-rollback";
            case "settings_change":
                return "badge-settings";
            case "server_add":
                return "badge-server-add";
            case "server_delete":
                return "badge-server-delete";
            case "credentials_change":
                return "badge-credentials";
            default:
                return "";
        }
    }

    function actionLabel(action) {
        switch (action) {
            case "login":
                return "Connexion";
            case "scan":
                return "Scan";
            case "update":
                return "Mise a jour";
            case "rollback":
                return "Rollback";
            case "settings_change":
                return "Parametre";
            case "server_add":
                return "Serveur +";
            case "server_delete":
                return "Serveur -";
            case "credentials_change":
                return "Credentials";
            default:
                return action;
        }
    }

    onMount(async () => {
        websocketStore.connect();
        await loadJobs();
        await loadAuditLogs();
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
                                {#if job.job_type === "rollback"}
                                    <span class="badge badge-rollback"
                                        >Rollback</span
                                    >
                                {/if}
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
                                <th>Type</th>
                                <th>Serveur</th>
                                <th>Image</th>
                                <th>Tag cible</th>
                                <th>Statut</th>
                                <th>Démarré</th>
                                <th>Durée</th>
                                <th>Erreur</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each completedJobs as job (job.id)}
                                <tr class:job-failed={job.status === "failed"}>
                                    <td
                                        ><strong>{job.container_name}</strong
                                        ></td
                                    >
                                    <td>
                                        {#if job.job_type === "rollback"}
                                            <span class="badge badge-rollback"
                                                >Rollback</span
                                            >
                                        {:else}
                                            <span class="badge badge-update"
                                                >Update</span
                                            >
                                        {/if}
                                    </td>
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
                                    <td>
                                        {#if job.previous_image && job.job_type !== "rollback"}
                                            <button
                                                class="btn btn-rollback"
                                                on:click={() =>
                                                    handleRollback(job)}
                                                disabled={rollingBack ===
                                                    job.id}
                                                title="Revenir à {job.previous_image}"
                                            >
                                                {#if rollingBack === job.id}
                                                    <span class="spinner-sm"
                                                    ></span>
                                                {:else}
                                                    <svg
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2"
                                                        width="14"
                                                        height="14"
                                                    >
                                                        <polyline
                                                            points="1 4 1 10 7 10"
                                                        ></polyline>
                                                        <path
                                                            d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"
                                                        ></path>
                                                    </svg>
                                                {/if}
                                                Rollback
                                            </button>
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

    <!-- Audit Logs -->
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
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"
                    ></path>
                </svg>
                Audit
                <span class="badge badge-info">{auditTotal}</span>
            </h2>
        </div>

        {#if auditLoading}
            <div class="card loading-card">
                <span class="spinner"></span> Chargement...
            </div>
        {:else if auditLogs.length === 0}
            <div class="card empty-state">
                <h3>Aucun log d'audit</h3>
                <p class="text-muted">Les actions seront enregistrees ici.</p>
            </div>
        {:else}
            <div class="table-container card">
                <table class="table">
                    <thead>
                        <tr>
                            <th>Date</th>
                            <th>Utilisateur</th>
                            <th>Action</th>
                            <th>Details</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each auditLogs as log (log.id)}
                            <tr>
                                <td class="date-cell"
                                    >{formatDate(log.created_at)}</td
                                >
                                <td><strong>{log.username}</strong></td>
                                <td>
                                    <span
                                        class="badge {actionBadgeClass(
                                            log.action,
                                        )}"
                                    >
                                        {actionLabel(log.action)}
                                    </span>
                                </td>
                                <td class="details-cell"
                                    >{log.details || "\u2014"}</td
                                >
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
            <div class="pagination">
                <button
                    class="btn btn-sm"
                    on:click={auditPrev}
                    disabled={auditOffset === 0}
                >
                    &larr; Precedent
                </button>
                <span class="text-muted text-sm">
                    {auditOffset + 1} - {Math.min(
                        auditOffset + auditLimit,
                        auditTotal,
                    )} sur {auditTotal}
                </span>
                <button
                    class="btn btn-sm"
                    on:click={auditNext}
                    disabled={auditOffset + auditLimit >= auditTotal}
                >
                    Suivant &rarr;
                </button>
            </div>
        {/if}
    </section>
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

    .badge-rollback {
        background: rgba(168, 85, 247, 0.15);
        color: #a855f7;
    }

    .badge-update {
        background: rgba(59, 130, 246, 0.15);
        color: #3b82f6;
    }

    .btn-rollback {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        padding: 4px 10px;
        font-size: 0.8rem;
        background: rgba(168, 85, 247, 0.1);
        color: #a855f7;
        border: 1px solid rgba(168, 85, 247, 0.3);
        border-radius: var(--border-radius);
        cursor: pointer;
        transition: all 0.2s;
        white-space: nowrap;
    }

    .btn-rollback:hover:not(:disabled) {
        background: rgba(168, 85, 247, 0.2);
        border-color: #a855f7;
    }

    .btn-rollback:disabled {
        opacity: 0.5;
        cursor: not-allowed;
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

    /* Audit log badges */
    .badge-login {
        background: rgba(34, 197, 94, 0.15);
        color: #22c55e;
    }

    .badge-scan {
        background: rgba(59, 130, 246, 0.15);
        color: #3b82f6;
    }

    .badge-settings {
        background: rgba(245, 158, 11, 0.15);
        color: #f59e0b;
    }

    .badge-server-add {
        background: rgba(34, 197, 94, 0.15);
        color: #22c55e;
    }

    .badge-server-delete {
        background: rgba(239, 68, 68, 0.15);
        color: #ef4444;
    }

    .badge-credentials {
        background: rgba(168, 85, 247, 0.15);
        color: #a855f7;
    }

    .details-cell {
        max-width: 400px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .pagination {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: var(--spacing-md);
        margin-top: var(--spacing-md);
    }

    .btn-sm {
        padding: 4px 12px;
        font-size: 0.8rem;
        background: var(--bg-card);
        color: var(--text-primary);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        cursor: pointer;
        transition: all 0.2s;
    }

    .btn-sm:hover:not(:disabled) {
        background: var(--bg-hover, rgba(255, 255, 255, 0.05));
        border-color: var(--color-info, #3b82f6);
    }

    .btn-sm:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }
</style>
