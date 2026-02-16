<script>
    import { onMount } from "svelte";
    import { getSettings, updateSetting } from "$lib/api.js";
    import { toasts } from "$lib/stores/toast.js";

    let settings = {};
    let loading = true;

    onMount(async () => {
        try {
            settings = await getSettings();
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
        padding: 6px 10px;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        color: var(--text-primary);
        font-size: 0.85rem;
        text-align: center;
    }

    .delay-input input:disabled {
        cursor: not-allowed;
    }

    .delay-unit {
        color: var(--text-muted);
        font-size: 0.8rem;
    }
</style>
