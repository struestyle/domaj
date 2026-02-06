<script>
    import { goto } from "$app/navigation";
    import { login, register, isAuthenticated } from "$lib/stores/auth.js";
    import { toasts } from "$lib/stores/toast.js";
    import { onMount } from "svelte";

    let username = "";
    let password = "";
    let confirmPassword = "";
    let loading = false;
    let isRegisterMode = false;
    let needsSetup = false;

    onMount(async () => {
        // Check if authenticated
        if ($isAuthenticated) {
            goto("/");
            return;
        }

        // Check if any users exist (first-time setup)
        try {
            const res = await fetch("/api/auth/login", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ username: "", password: "" }),
            });
            // If we get a specific error about no users, show setup mode
            const text = await res.text();
            if (text.includes("Invalid credentials")) {
                needsSetup = false;
            }
        } catch {
            // Server might not be running
        }
    });

    async function handleSubmit() {
        if (loading) return;

        if (isRegisterMode && password !== confirmPassword) {
            toasts.error("Les mots de passe ne correspondent pas");
            return;
        }

        loading = true;

        try {
            if (isRegisterMode) {
                await register(username, password);
                toasts.success("Compte créé avec succès !");
            } else {
                await login(username, password);
                toasts.success("Connexion réussie !");
            }
            goto("/");
        } catch (err) {
            toasts.error(err.message || "Erreur de connexion");
        } finally {
            loading = false;
        }
    }

    function toggleMode() {
        isRegisterMode = !isRegisterMode;
        confirmPassword = "";
    }
</script>

<svelte:head>
    <title>{isRegisterMode ? "Inscription" : "Connexion"} - Domaj</title>
</svelte:head>

<div class="login-container">
    <div class="login-card">
        <div class="login-header">
            <div class="logo">
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path
                        d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                    ></path>
                    <polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline>
                    <line x1="12" y1="22.08" x2="12" y2="12"></line>
                </svg>
            </div>
            <h1>Domaj</h1>
            <p class="subtitle">
                {isRegisterMode ? "Créer un compte" : "Connexion"}
            </p>
        </div>

        <form on:submit|preventDefault={handleSubmit}>
            <div class="form-group">
                <label for="username">Nom d'utilisateur</label>
                <input
                    type="text"
                    id="username"
                    class="input"
                    bind:value={username}
                    placeholder="admin"
                    required
                    autocomplete="username"
                />
            </div>

            <div class="form-group">
                <label for="password">Mot de passe</label>
                <input
                    type="password"
                    id="password"
                    class="input"
                    bind:value={password}
                    placeholder="••••••••"
                    required
                    autocomplete={isRegisterMode
                        ? "new-password"
                        : "current-password"}
                />
            </div>

            {#if isRegisterMode}
                <div class="form-group">
                    <label for="confirmPassword"
                        >Confirmer le mot de passe</label
                    >
                    <input
                        type="password"
                        id="confirmPassword"
                        class="input"
                        bind:value={confirmPassword}
                        placeholder="••••••••"
                        required
                        autocomplete="new-password"
                    />
                </div>
            {/if}

            <button
                type="submit"
                class="btn btn-primary btn-full"
                disabled={loading}
            >
                {#if loading}
                    Chargement...
                {:else}
                    {isRegisterMode ? "Créer le compte" : "Se connecter"}
                {/if}
            </button>
        </form>

        <div class="toggle-mode">
            <button type="button" class="link-button" on:click={toggleMode}>
                {isRegisterMode
                    ? "Déjà un compte ? Se connecter"
                    : "Pas de compte ? S'inscrire"}
            </button>
        </div>
    </div>
</div>

<style>
    .login-container {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--bg-primary);
        padding: var(--spacing-lg);
    }

    .login-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius-lg);
        padding: var(--spacing-xl);
        width: 100%;
        max-width: 400px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    }

    .login-header {
        text-align: center;
        margin-bottom: var(--spacing-xl);
    }

    .logo {
        width: 64px;
        height: 64px;
        margin: 0 auto var(--spacing-md);
    }

    .logo svg {
        width: 100%;
        height: 100%;
        stroke: var(--color-primary);
    }

    .login-header h1 {
        font-size: 2rem;
        font-weight: 700;
        background: linear-gradient(
            135deg,
            var(--color-primary) 0%,
            var(--color-secondary) 100%
        );
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        background-clip: text;
        margin-bottom: var(--spacing-xs);
    }

    .subtitle {
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .form-group {
        margin-bottom: var(--spacing-md);
    }

    .form-group label {
        display: block;
        margin-bottom: var(--spacing-xs);
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-secondary);
    }

    .btn-full {
        width: 100%;
        margin-top: var(--spacing-md);
    }

    .toggle-mode {
        text-align: center;
        margin-top: var(--spacing-lg);
    }

    .link-button {
        background: none;
        border: none;
        color: var(--color-primary);
        cursor: pointer;
        font-size: 0.875rem;
        padding: 0;
    }

    .link-button:hover {
        text-decoration: underline;
    }
</style>
