<script>
    import { toasts } from "$lib/stores/toast.js";
</script>

{#if $toasts.length > 0}
    <div class="toast-container">
        {#each $toasts as toast (toast.id)}
            <div class="toast toast-{toast.type}" role="alert">
                <div class="toast-content">
                    {#if toast.type === "success"}
                        <svg
                            class="toast-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                            <polyline points="22 4 12 14.01 9 11.01"></polyline>
                        </svg>
                    {:else if toast.type === "error"}
                        <svg
                            class="toast-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <circle cx="12" cy="12" r="10"></circle>
                            <line x1="15" y1="9" x2="9" y2="15"></line>
                            <line x1="9" y1="9" x2="15" y2="15"></line>
                        </svg>
                    {:else}
                        <svg
                            class="toast-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <circle cx="12" cy="12" r="10"></circle>
                            <line x1="12" y1="16" x2="12" y2="12"></line>
                            <line x1="12" y1="8" x2="12.01" y2="8"></line>
                        </svg>
                    {/if}
                    <span class="toast-message">{toast.message}</span>
                </div>
                <button
                    class="toast-close"
                    on:click={() => toasts.dismiss(toast.id)}
                    aria-label="Fermer"
                >
                    <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <line x1="18" y1="6" x2="6" y2="18"></line>
                        <line x1="6" y1="6" x2="18" y2="18"></line>
                    </svg>
                </button>
            </div>
        {/each}
    </div>
{/if}

<style>
    .toast-container {
        position: fixed;
        bottom: 24px;
        right: 24px;
        z-index: 9999;
        display: flex;
        flex-direction: column;
        gap: 12px;
        max-width: 400px;
    }

    .toast {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 14px 16px;
        border-radius: var(--border-radius-md, 8px);
        background: var(--bg-card, #1e1e2e);
        border: 1px solid var(--border-color, #333);
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
        animation: slideIn 0.3s ease-out;
    }

    @keyframes slideIn {
        from {
            opacity: 0;
            transform: translateX(100%);
        }
        to {
            opacity: 1;
            transform: translateX(0);
        }
    }

    .toast-success {
        border-left: 4px solid var(--color-success, #10b981);
    }

    .toast-error {
        border-left: 4px solid var(--color-error, #ef4444);
    }

    .toast-info {
        border-left: 4px solid var(--color-primary, #6366f1);
    }

    .toast-content {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .toast-icon {
        width: 20px;
        height: 20px;
        flex-shrink: 0;
    }

    .toast-success .toast-icon {
        stroke: var(--color-success, #10b981);
    }

    .toast-error .toast-icon {
        stroke: var(--color-error, #ef4444);
    }

    .toast-info .toast-icon {
        stroke: var(--color-primary, #6366f1);
    }

    .toast-message {
        font-size: 0.9rem;
        color: var(--text-primary, #fff);
    }

    .toast-close {
        background: none;
        border: none;
        cursor: pointer;
        padding: 4px;
        margin-left: 12px;
        opacity: 0.6;
        transition: opacity 0.2s;
    }

    .toast-close:hover {
        opacity: 1;
    }

    .toast-close svg {
        width: 16px;
        height: 16px;
        stroke: var(--text-secondary, #888);
    }

    @media (max-width: 480px) {
        .toast-container {
            left: 16px;
            right: 16px;
            bottom: 16px;
            max-width: none;
        }
    }
</style>
