<script>
    import { onMount } from "svelte";
    import {
        getRegistries,
        addRegistryCredential,
        updateRegistryCredential,
        deleteRegistryCredential,
    } from "$lib/api.js";
    import { toasts } from "$lib/stores/toast.js";

    let registries = [];
    let loading = true;
    let error = null;

    // Form state
    let showForm = false;
    let editingId = null;
    let formData = { host: "", username: "", password: "" };
    let formError = null;
    let formLoading = false;

    // Delete confirmation
    let deletingId = null;

    onMount(async () => {
        await loadRegistries();
    });

    async function loadRegistries() {
        loading = true;
        error = null;
        try {
            registries = await getRegistries();
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    }

    function openAddForm() {
        editingId = null;
        formData = { host: "", username: "", password: "" };
        formError = null;
        showForm = true;
    }

    function openEditForm(registry) {
        editingId = registry.credential_id;
        formData = { host: registry.host, username: "", password: "" };
        formError = null;
        showForm = true;
    }

    function closeForm() {
        showForm = false;
        editingId = null;
        formData = { host: "", username: "", password: "" };
        formError = null;
    }

    async function handleSubmit() {
        if (!formData.host || !formData.username || !formData.password) {
            formError = "Tous les champs sont requis";
            return;
        }
        formLoading = true;
        formError = null;
        try {
            if (editingId) {
                await updateRegistryCredential(editingId, formData);
                toasts.success("Identifiants mis à jour");
            } else {
                await addRegistryCredential(formData);
                toasts.success("Identifiants ajoutés");
            }
            closeForm();
            await loadRegistries();
        } catch (e) {
            formError = e.message || "Erreur lors de l'enregistrement";
        } finally {
            formLoading = false;
        }
    }

    async function handleDelete(id) {
        try {
            await deleteRegistryCredential(id);
            toasts.success("Identifiants supprimés");
            deletingId = null;
            await loadRegistries();
        } catch (e) {
            toasts.error(e.message || "Erreur lors de la suppression");
        }
    }

    function getStatusLabel(status) {
        switch (status) {
            case "accessible":
                return "Accessible";
            case "auth_failed":
                return "Auth échouée";
            case "unreachable":
                return "Injoignable";
            default:
                return status;
        }
    }

    function getStatusClass(status) {
        switch (status) {
            case "accessible":
                return "badge-success";
            case "auth_failed":
                return "badge-warning";
            case "unreachable":
                return "badge-danger";
            default:
                return "";
        }
    }
</script>

<svelte:head>
    <title>Registres - Domaj</title>
</svelte:head>

<div class="container">
    <header class="page-header">
        <div>
            <h1>Registres de conteneurs</h1>
            <p class="text-muted">
                Registres détectés et gestion des identifiants
            </p>
        </div>
        <div class="header-actions">
            <button class="btn btn-primary" on:click={openAddForm}>
                <svg
                    class="btn-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <line x1="12" y1="5" x2="12" y2="19"></line>
                    <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
                Ajouter des identifiants
            </button>
            <button
                class="btn btn-secondary"
                on:click={loadRegistries}
                disabled={loading}
            >
                <svg
                    class="btn-icon"
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
                Rafraîchir
            </button>
        </div>
    </header>

    <!-- Credential Form Modal -->
    {#if showForm}
        <div
            class="modal-overlay"
            on:click={closeForm}
            on:keydown={(e) => e.key === "Escape" && closeForm()}
            role="dialog"
            aria-modal="true"
            tabindex="-1"
        >
            <!-- svelte-ignore a11y-click-events-have-key-events -->
            <div class="modal" on:click|stopPropagation role="document">
                <div class="modal-header">
                    <h2>
                        {editingId
                            ? "Modifier les identifiants"
                            : "Ajouter des identifiants"}
                    </h2>
                    <button
                        class="btn-close"
                        on:click={closeForm}
                        aria-label="Fermer">&times;</button
                    >
                </div>
                <form on:submit|preventDefault={handleSubmit}>
                    {#if formError}
                        <div class="form-error">{formError}</div>
                    {/if}
                    <div class="form-group">
                        <label for="cred-host">Hôte du registre</label>
                        <input
                            id="cred-host"
                            type="text"
                            bind:value={formData.host}
                            placeholder="hb.example.com"
                            disabled={!!editingId}
                        />
                    </div>
                    <div class="form-group">
                        <label for="cred-user">Nom d'utilisateur</label>
                        <input
                            id="cred-user"
                            type="text"
                            bind:value={formData.username}
                            placeholder="admin"
                        />
                    </div>
                    <div class="form-group">
                        <label for="cred-pass">Mot de passe</label>
                        <input
                            id="cred-pass"
                            type="password"
                            bind:value={formData.password}
                            placeholder="••••••••"
                        />
                    </div>
                    <div class="form-actions">
                        <button
                            type="button"
                            class="btn btn-secondary"
                            on:click={closeForm}
                        >
                            Annuler
                        </button>
                        <button
                            type="submit"
                            class="btn btn-primary"
                            disabled={formLoading}
                        >
                            {formLoading
                                ? "Enregistrement..."
                                : editingId
                                  ? "Modifier"
                                  : "Ajouter"}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    {/if}

    <!-- Delete Confirmation Modal -->
    {#if deletingId}
        <div
            class="modal-overlay"
            on:click={() => (deletingId = null)}
            on:keydown={(e) => e.key === "Escape" && (deletingId = null)}
            role="dialog"
            aria-modal="true"
            tabindex="-1"
        >
            <!-- svelte-ignore a11y-click-events-have-key-events -->
            <div
                class="modal modal-sm"
                on:click|stopPropagation
                role="document"
            >
                <div class="modal-header">
                    <h2>Confirmer la suppression</h2>
                </div>
                <p class="text-muted">
                    Voulez-vous vraiment supprimer ces identifiants ?
                </p>
                <div class="form-actions">
                    <button
                        class="btn btn-secondary"
                        on:click={() => (deletingId = null)}>Annuler</button
                    >
                    <button
                        class="btn btn-danger"
                        on:click={() => handleDelete(deletingId)}
                        >Supprimer</button
                    >
                </div>
            </div>
        </div>
    {/if}

    {#if loading}
        <div class="skeleton" style="height: 200px;"></div>
    {:else if error}
        <div class="card error-card">
            <p>⚠️ {error}</p>
        </div>
    {:else if registries.length === 0}
        <div class="empty-state card">
            <div class="empty-icon">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                    width="64"
                    height="64"
                >
                    <path
                        d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                    ></path>
                    <polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline>
                    <line x1="12" y1="22.08" x2="12" y2="12"></line>
                </svg>
            </div>
            <p class="text-muted">
                Aucun registre détecté. Synchronisez vos serveurs ou ajoutez des
                identifiants manuellement.
            </p>
        </div>
    {:else}
        <div class="table-wrapper card">
            <table class="table">
                <thead>
                    <tr>
                        <th>Statut</th>
                        <th>Registre</th>
                        <th>Identifiants</th>
                        <th>Conteneurs</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {#each registries as registry}
                        <tr
                            class:row-env={registry.credential_source === "env"}
                        >
                            <td>
                                <span
                                    class="badge {getStatusClass(
                                        registry.status,
                                    )}"
                                >
                                    {getStatusLabel(registry.status)}
                                </span>
                            </td>
                            <td>
                                <code class="registry-host"
                                    >{registry.host}</code
                                >
                            </td>
                            <td>
                                {#if registry.has_credentials}
                                    <span class="credential-badge configured">
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            width="14"
                                            height="14"
                                        >
                                            <rect
                                                x="3"
                                                y="11"
                                                width="18"
                                                height="11"
                                                rx="2"
                                                ry="2"
                                            ></rect>
                                            <path d="M7 11V7a5 5 0 0 1 10 0v4"
                                            ></path>
                                        </svg>
                                        {#if registry.credential_source === "env"}
                                            <span class="source-tag env"
                                                >env</span
                                            >
                                        {:else}
                                            <span class="source-tag db">ui</span
                                            >
                                        {/if}
                                    </span>
                                {:else}
                                    <span
                                        class="credential-badge not-configured"
                                    >
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            width="14"
                                            height="14"
                                        >
                                            <rect
                                                x="3"
                                                y="11"
                                                width="18"
                                                height="11"
                                                rx="2"
                                                ry="2"
                                            ></rect>
                                            <path d="M7 11V7a5 5 0 0 1 10 0v4"
                                            ></path>
                                        </svg>
                                        Aucun
                                    </span>
                                {/if}
                            </td>
                            <td>
                                <span class="container-count"
                                    >{registry.container_count}</span
                                >
                            </td>
                            <td>
                                {#if registry.credential_source === "db"}
                                    <div class="action-btns">
                                        <button
                                            class="btn btn-sm btn-secondary"
                                            on:click={() =>
                                                openEditForm(registry)}
                                            title="Modifier"
                                        >
                                            <svg
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                width="14"
                                                height="14"
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
                                            class="btn btn-sm btn-danger"
                                            on:click={() =>
                                                (deletingId =
                                                    registry.credential_id)}
                                            title="Supprimer"
                                        >
                                            <svg
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                width="14"
                                                height="14"
                                            >
                                                <polyline points="3 6 5 6 21 6"
                                                ></polyline>
                                                <path
                                                    d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                                ></path>
                                            </svg>
                                        </button>
                                    </div>
                                {:else if !registry.has_credentials}
                                    <button
                                        class="btn btn-sm btn-primary"
                                        on:click={() => {
                                            formData.host = registry.host;
                                            openAddForm();
                                            formData.host = registry.host;
                                        }}
                                        title="Configurer"
                                    >
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2"
                                            width="14"
                                            height="14"
                                        >
                                            <line x1="12" y1="5" x2="12" y2="19"
                                            ></line>
                                            <line x1="5" y1="12" x2="19" y2="12"
                                            ></line>
                                        </svg>
                                        Configurer
                                    </button>
                                {:else}
                                    <span class="text-muted">—</span>
                                {/if}
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
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

    .header-actions {
        display: flex;
        gap: var(--spacing-sm);
    }

    .btn-icon {
        width: 16px;
        height: 16px;
        margin-right: 6px;
        vertical-align: middle;
    }

    .table-wrapper {
        overflow: hidden;
    }

    .registry-host {
        font-size: 0.875rem;
        background: var(--bg-primary);
        padding: 4px 8px;
        border-radius: 4px;
        font-weight: 500;
    }

    .credential-badge {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-size: 0.8125rem;
        padding: 4px 10px;
        border-radius: 4px;
    }

    .credential-badge.configured {
        background: rgba(46, 204, 113, 0.1);
        color: var(--color-success);
    }

    .credential-badge.not-configured {
        background: rgba(255, 255, 255, 0.05);
        color: var(--text-secondary);
    }

    .source-tag {
        font-size: 0.6875rem;
        font-weight: 600;
        text-transform: uppercase;
        padding: 1px 5px;
        border-radius: 3px;
        letter-spacing: 0.5px;
    }

    .source-tag.env {
        background: rgba(155, 89, 182, 0.15);
        color: #b07dd0;
    }

    .source-tag.db {
        background: rgba(52, 152, 219, 0.15);
        color: #5dade2;
    }

    .container-count {
        font-weight: 600;
        font-size: 0.9375rem;
    }

    .action-btns {
        display: flex;
        gap: 6px;
    }

    .btn-sm {
        padding: 4px 8px;
        font-size: 0.8125rem;
    }

    .btn-danger {
        background: rgba(231, 76, 60, 0.15);
        color: var(--color-danger);
        border: 1px solid rgba(231, 76, 60, 0.3);
    }

    .btn-danger:hover {
        background: rgba(231, 76, 60, 0.25);
    }

    tr.row-env {
        opacity: 0.5;
    }

    tr.row-env:hover {
        opacity: 0.7;
    }

    /* Modal */
    .modal-overlay {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
        backdrop-filter: blur(4px);
    }

    .modal {
        background: var(--bg-secondary);
        border-radius: 12px;
        padding: var(--spacing-lg);
        width: 100%;
        max-width: 460px;
        border: 1px solid var(--border-color);
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    }

    .modal-sm {
        max-width: 380px;
    }

    .modal-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: var(--spacing-lg);
    }

    .modal-header h2 {
        font-size: 1.25rem;
        font-weight: 600;
        margin: 0;
    }

    .btn-close {
        background: none;
        border: none;
        color: var(--text-secondary);
        font-size: 1.5rem;
        cursor: pointer;
        padding: 0 4px;
        line-height: 1;
    }

    .btn-close:hover {
        color: var(--text-primary);
    }

    .form-group {
        margin-bottom: var(--spacing-md);
    }

    .form-group label {
        display: block;
        font-size: 0.875rem;
        font-weight: 500;
        margin-bottom: var(--spacing-xs);
        color: var(--text-secondary);
    }

    .form-group input {
        width: 100%;
        padding: 10px 12px;
        border: 1px solid var(--border-color);
        border-radius: 8px;
        background: var(--bg-primary);
        color: var(--text-primary);
        font-size: 0.9375rem;
        box-sizing: border-box;
    }

    .form-group input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(52, 152, 219, 0.15);
    }

    .form-group input:disabled {
        opacity: 0.5;
    }

    .form-error {
        background: rgba(231, 76, 60, 0.1);
        color: var(--color-danger);
        padding: 10px 14px;
        border-radius: 8px;
        font-size: 0.875rem;
        margin-bottom: var(--spacing-md);
    }

    .form-actions {
        display: flex;
        justify-content: flex-end;
        gap: var(--spacing-sm);
        margin-top: var(--spacing-lg);
    }

    .empty-state {
        text-align: center;
        padding: var(--spacing-xl);
    }

    .empty-icon {
        margin-bottom: var(--spacing-md);
        color: var(--text-muted);
    }

    .error-card {
        padding: var(--spacing-lg);
        color: var(--color-danger);
    }

    @media (max-width: 768px) {
        .page-header {
            flex-direction: column;
            gap: var(--spacing-md);
        }

        .header-actions {
            flex-direction: column;
            width: 100%;
        }
    }
</style>
