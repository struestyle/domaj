<script>
    import { onMount } from "svelte";
    import {
        getSettings,
        updateSetting,
        testDockerCredentials,
    } from "$lib/api.js";
    import { toasts } from "$lib/stores/toast.js";
    import { theme, toggleTheme } from "$lib/stores/theme.js";

    let settings = {};
    let loading = true;
    let dockerUsername = "";
    let dockerPassword = "";
    let savingDocker = false;

    onMount(async () => {
        try {
            settings = await getSettings();
            dockerUsername = settings.docker_username?.value ?? "";
        } catch (e) {
            toasts.error("Impossible de charger les paramètres");
        } finally {
            loading = false;
        }
    });

    async function toggleAutoRollback(e) {
        try {
            await updateSetting("auto_rollback", e.target.checked);
            settings = await getSettings();
            toasts.success(
                `Auto-rollback ${e.target.checked ? "activé" : "désactivé"}`,
            );
        } catch (err) {
            toasts.error(err.message);
            e.target.checked = !e.target.checked;
        }
    }

    async function updateDelay(e) {
        const val = parseInt(e.target.value);
        if (val < 5 || val > 300) {
            toasts.error("La valeur doit être entre 5 et 300 secondes");
            e.target.value = settings.auto_rollback_delay?.value ?? 30;
            return;
        }
        try {
            await updateSetting("auto_rollback_delay", val);
            settings = await getSettings();
            toasts.success(`Délai de vérification mis à jour : ${val}s`);
        } catch (err) {
            toasts.error(err.message);
        }
    }

    async function saveDockerCredentials() {
        savingDocker = true;
        try {
            await updateSetting("docker_username", dockerUsername);
            await updateSetting("docker_password", dockerPassword);
            // Test the credentials against Docker Hub
            try {
                const result = await testDockerCredentials();
                toasts.success(
                    result.message || "Identifiants Docker Hub validés ✓",
                );
            } catch (testErr) {
                toasts.error(
                    testErr.message ||
                        "Identifiants sauvegardés mais invalides",
                );
            }
            settings = await getSettings();
            dockerPassword = "";
        } catch (err) {
            toasts.error(err.message);
        } finally {
            savingDocker = false;
        }
    }
</script>

<svelte:head>
    <title>Paramètres - Domaj</title>
</svelte:head>

<div class="container">
    <div class="page-header">
        <h1>
            <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                width="28"
                height="28"
                style="vertical-align: middle; margin-right: 10px;"
            >
                <circle cx="12" cy="12" r="3"></circle>
                <path
                    d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
                ></path>
            </svg>
            Paramètres
        </h1>
    </div>

    {#if loading}
        <div class="loading">Chargement...</div>
    {:else}
        <!-- Theme Section -->
        <div class="settings-section">
            <h2 class="section-title">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    width="20"
                    height="20"
                >
                    {#if $theme === "dark"}
                        <path
                            d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"
                        ></path>
                    {:else}
                        <circle cx="12" cy="12" r="5"></circle>
                        <line x1="12" y1="1" x2="12" y2="3"></line>
                        <line x1="12" y1="21" x2="12" y2="23"></line>
                        <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
                        <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"
                        ></line>
                        <line x1="1" y1="12" x2="3" y2="12"></line>
                        <line x1="21" y1="12" x2="23" y2="12"></line>
                        <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
                        <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
                    {/if}
                </svg>
                Apparence
            </h2>
            <p class="section-description">
                Choisissez le thème de l'interface.
            </p>

            <div class="settings-card card">
                <div class="setting-row">
                    <div class="setting-info">
                        <span class="setting-label">Mode sombre</span>
                        <p class="setting-description">
                            Basculer entre le thème clair et le thème sombre
                        </p>
                    </div>
                    <div class="setting-control">
                        <label class="toggle">
                            <input
                                type="checkbox"
                                checked={$theme === "dark"}
                                on:change={toggleTheme}
                            />
                            <span class="toggle-slider"></span>
                        </label>
                    </div>
                </div>
            </div>
        </div>

        <!-- Auto-Rollback Section -->
        <div class="settings-section">
            <h2 class="section-title">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    width="20"
                    height="20"
                >
                    <polyline points="1 4 1 10 7 10"></polyline>
                    <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>
                </svg>
                Auto-Rollback
            </h2>
            <p class="section-description">
                Après une mise à jour, Domaj vérifie que le conteneur reste en
                fonctionnement. Si le conteneur tombe, l'image précédente est
                automatiquement restaurée.
            </p>

            <div class="settings-card card">
                <div class="setting-row">
                    <div class="setting-info">
                        <span class="setting-label"
                            >Activer l'auto-rollback</span
                        >
                        <p class="setting-description">
                            Revient automatiquement à l'image précédente si le
                            conteneur tombe après une mise à jour
                        </p>
                    </div>
                    <div class="setting-control">
                        {#if settings.auto_rollback?.locked}
                            <span
                                class="lock-badge"
                                title="Verrouillé par variable d'environnement AUTO_ROLLBACK"
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
                                    <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
                                </svg>
                                ENV
                            </span>
                        {/if}
                        <label
                            class="toggle"
                            class:locked={settings.auto_rollback?.locked}
                        >
                            <input
                                type="checkbox"
                                checked={settings.auto_rollback?.value ?? true}
                                disabled={settings.auto_rollback?.locked}
                                on:change={toggleAutoRollback}
                            />
                            <span class="toggle-slider"></span>
                        </label>
                    </div>
                </div>

                <div class="setting-row">
                    <div class="setting-info">
                        <span class="setting-label">Délai de vérification</span>
                        <p class="setting-description">
                            Temps d'attente (en secondes) avant de vérifier
                            l'état du conteneur après mise à jour
                        </p>
                    </div>
                    <div class="setting-control">
                        {#if settings.auto_rollback_delay?.locked}
                            <span
                                class="lock-badge"
                                title="Verrouillé par variable d'environnement AUTO_ROLLBACK_DELAY"
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
                                    <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
                                </svg>
                                ENV
                            </span>
                        {/if}
                        <div
                            class="delay-input"
                            class:locked={settings.auto_rollback_delay?.locked}
                        >
                            <input
                                type="number"
                                min="5"
                                max="300"
                                value={settings.auto_rollback_delay?.value ??
                                    30}
                                disabled={settings.auto_rollback_delay?.locked}
                                on:change={updateDelay}
                            />
                            <span class="delay-unit">secondes</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Docker Hub Section -->
        <div class="settings-section">
            <h2 class="section-title">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    width="20"
                    height="20"
                >
                    <path
                        d="M22 12.5c0-.3-.2-.6-.4-.8-.3-.2-.6-.3-1-.3-.1-1-.5-1.9-1.2-2.6.2-.7.2-1.4 0-2.1-.1-.2-.3-.3-.5-.3-.4 0-1 .2-1.8.7-.7-.3-1.4-.5-2.1-.5V6c0-.3-.1-.5-.3-.7-.2-.1-.5-.1-.7 0-1.1.6-1.8 1.5-2.1 2.5H11c-.7 0-1.4.1-2 .4-.7-.5-1.3-.7-1.7-.7-.2 0-.4.1-.5.3-.3.7-.3 1.4 0 2.1-.7.7-1.1 1.6-1.2 2.6-.4 0-.7.1-1 .3-.2.2-.4.5-.4.8 0 .6.4 1.1 1.1 1.4.2 1.4 1.1 2.7 2.4 3.4.4 1.1 1.4 1.8 2.6 2 .4.2.8.3 1.3.3h4.8c.5 0 .9-.1 1.3-.3 1.2-.2 2.2-.9 2.6-2 1.3-.7 2.2-2 2.4-3.4.7-.3 1.1-.8 1.1-1.4z"
                    ></path>
                </svg>
                Docker Hub
            </h2>
            <p class="section-description">
                Configurez vos identifiants Docker Hub pour augmenter la limite
                de requêtes API (200/6h au lieu de 100 en anonyme).
            </p>

            <div class="settings-card card">
                <div class="setting-row">
                    <div class="setting-info">
                        <span class="setting-label">Nom d'utilisateur</span>
                        <p class="setting-description">
                            Votre identifiant Docker Hub
                        </p>
                    </div>
                    <div class="setting-control">
                        {#if settings.docker_username?.locked}
                            <span
                                class="lock-badge"
                                title="Verrouillé par variable d'environnement DOCKER_USERNAME"
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
                                    <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
                                </svg>
                                ENV
                            </span>
                        {/if}
                        <input
                            type="text"
                            class="text-input"
                            class:locked={settings.docker_username?.locked}
                            placeholder="username"
                            value={settings.docker_username?.value ?? ""}
                            disabled={settings.docker_username?.locked}
                            on:input={(e) => (dockerUsername = e.target.value)}
                        />
                    </div>
                </div>

                <div class="setting-row">
                    <div class="setting-info">
                        <span class="setting-label">Mot de passe / Token</span>
                        <p class="setting-description">
                            Votre mot de passe ou Personal Access Token Docker
                            Hub
                        </p>
                    </div>
                    <div class="setting-control">
                        {#if settings.docker_password?.locked}
                            <span
                                class="lock-badge"
                                title="Verrouillé par variable d'environnement DOCKER_PASSWORD"
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
                                    <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
                                </svg>
                                ENV
                            </span>
                        {/if}
                        <input
                            type="password"
                            class="text-input"
                            class:locked={settings.docker_password?.locked}
                            placeholder="••••••••"
                            value={dockerPassword}
                            disabled={settings.docker_password?.locked}
                            on:input={(e) => (dockerPassword = e.target.value)}
                        />
                    </div>
                </div>

                {#if !settings.docker_username?.locked && !settings.docker_password?.locked}
                    <div class="setting-row docker-actions">
                        <button
                            class="btn btn-primary"
                            disabled={savingDocker}
                            on:click={saveDockerCredentials}
                        >
                            {#if savingDocker}
                                <span class="spinner-small"></span>
                            {:else}
                                <svg
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    width="16"
                                    height="16"
                                >
                                    <path
                                        d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"
                                    ></path>
                                    <polyline points="17 21 17 13 7 13 7 21"
                                    ></polyline>
                                    <polyline points="7 3 7 8 15 8"></polyline>
                                </svg>
                            {/if}
                            Sauvegarder
                        </button>
                        {#if settings.docker_username?.value && settings.docker_password?.value === "••••••••"}
                            <span class="docker-status connected">
                                <svg
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    width="14"
                                    height="14"
                                >
                                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"
                                    ></path>
                                    <polyline points="22 4 12 14.01 9 11.01"
                                    ></polyline>
                                </svg>
                                Configuré
                            </span>
                        {:else}
                            <span class="docker-status not-configured">
                                Non configuré
                            </span>
                        {/if}
                    </div>
                {:else}
                    <div class="setting-row docker-actions">
                        <span class="docker-status connected">
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                width="14"
                                height="14"
                            >
                                <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"
                                ></path>
                                <polyline points="22 4 12 14.01 9 11.01"
                                ></polyline>
                            </svg>
                            Configuré via variables d'environnement
                        </span>
                    </div>
                {/if}
            </div>
        </div>
    {/if}
</div>

<style>
    .page-header {
        margin-bottom: var(--spacing-xl);
    }

    .page-header h1 {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary);
        display: flex;
        align-items: center;
    }

    .loading {
        text-align: center;
        padding: var(--spacing-xl);
        color: var(--text-muted);
    }

    .settings-section {
        margin-bottom: var(--spacing-xl);
    }

    .section-title {
        font-size: 1.1rem;
        font-weight: 600;
        color: var(--text-primary);
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
        margin-bottom: var(--spacing-xs);
    }

    .section-description {
        color: var(--text-muted);
        font-size: 0.85rem;
        margin-bottom: var(--spacing-md);
        line-height: 1.5;
    }

    .settings-card {
        padding: var(--spacing-lg);
    }

    .setting-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: var(--spacing-md) 0;
    }

    .setting-row + .setting-row {
        border-top: 1px solid var(--border-color);
    }

    .setting-info {
        flex: 1;
        margin-right: var(--spacing-lg);
    }

    .setting-label {
        font-weight: 600;
        color: var(--text-primary);
        font-size: 0.95rem;
    }

    .setting-description {
        color: var(--text-muted);
        font-size: 0.8rem;
        margin-top: 4px;
    }

    .setting-control {
        display: flex;
        align-items: center;
        gap: var(--spacing-sm);
    }

    .lock-badge {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        padding: 2px 8px;
        background: rgba(251, 191, 36, 0.15);
        color: var(--color-warning);
        border-radius: var(--radius-sm);
        font-size: 0.7rem;
        font-weight: 600;
        white-space: nowrap;
    }

    /* Toggle switch */
    .toggle {
        position: relative;
        display: inline-block;
        width: 44px;
        height: 24px;
        cursor: pointer;
        flex-shrink: 0;
    }

    .toggle.locked {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .toggle input {
        opacity: 0;
        width: 0;
        height: 0;
    }

    .toggle-slider {
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: var(--bg-tertiary);
        border-radius: 24px;
        transition: all var(--transition-fast);
    }

    .toggle-slider::after {
        content: "";
        position: absolute;
        width: 18px;
        height: 18px;
        left: 3px;
        bottom: 3px;
        background: var(--text-muted);
        border-radius: 50%;
        transition: all var(--transition-fast);
    }

    .toggle input:checked + .toggle-slider {
        background: rgba(34, 197, 94, 0.3);
    }

    .toggle input:checked + .toggle-slider::after {
        transform: translateX(20px);
        background: var(--color-success);
    }

    .delay-input {
        display: flex;
        align-items: center;
        gap: 6px;
    }

    .delay-input.locked {
        opacity: 0.5;
    }

    .delay-input input {
        width: 70px;
        padding: 8px 10px;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md, 8px);
        color: var(--text-primary);
        font-size: 0.85rem;
        text-align: center;
        transition:
            border-color var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .delay-input input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
    }

    .delay-input input:disabled {
        cursor: not-allowed;
    }

    .delay-unit {
        color: var(--text-muted);
        font-size: 0.8rem;
    }

    /* Text input for Docker Hub */
    .text-input {
        padding: 8px 14px;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md, 8px);
        color: var(--text-primary);
        font-size: 0.85rem;
        min-width: 220px;
        transition:
            border-color var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .text-input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
    }

    .text-input.locked {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .text-input:disabled {
        cursor: not-allowed;
    }

    .docker-actions {
        display: flex;
        align-items: center;
        gap: var(--spacing-md);
        justify-content: flex-start;
    }

    .btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 8px 16px;
        border: none;
        border-radius: var(--radius-sm);
        font-size: 0.85rem;
        font-weight: 600;
        cursor: pointer;
        transition: all var(--transition-fast);
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover:not(:disabled) {
        filter: brightness(1.1);
    }

    .btn-primary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .docker-status {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        font-size: 0.8rem;
        font-weight: 500;
    }

    .docker-status.connected {
        color: var(--color-success);
    }

    .docker-status.not-configured {
        color: var(--text-muted);
    }

    .spinner-small {
        width: 14px;
        height: 14px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top: 2px solid white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
</style>
