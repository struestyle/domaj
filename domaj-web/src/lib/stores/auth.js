import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

const TOKEN_KEY = 'domaj_token';
const USER_KEY = 'domaj_user';

// Initialize from localStorage
function getStoredToken() {
    if (browser) {
        return localStorage.getItem(TOKEN_KEY);
    }
    return null;
}

function getStoredUser() {
    if (browser) {
        const user = localStorage.getItem(USER_KEY);
        return user ? JSON.parse(user) : null;
    }
    return null;
}

// Create writable stores
export const token = writable(getStoredToken());
export const user = writable(getStoredUser());

// Derived store for authentication status
export const isAuthenticated = derived(token, $token => !!$token);

// Subscribe to changes and persist to localStorage
if (browser) {
    token.subscribe(value => {
        if (value) {
            localStorage.setItem(TOKEN_KEY, value);
        } else {
            localStorage.removeItem(TOKEN_KEY);
        }
    });

    user.subscribe(value => {
        if (value) {
            localStorage.setItem(USER_KEY, JSON.stringify(value));
        } else {
            localStorage.removeItem(USER_KEY);
        }
    });
}

// Auth functions
export async function login(username, password) {
    const response = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, password })
    });

    if (!response.ok) {
        const error = await response.text();
        throw new Error(error || 'Login failed');
    }

    const data = await response.json();
    token.set(data.token);
    user.set(data.user);
    return data;
}

export async function register(username, password) {
    const response = await fetch('/api/auth/register', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, password })
    });

    if (!response.ok) {
        const error = await response.text();
        throw new Error(error || 'Registration failed');
    }

    const data = await response.json();
    token.set(data.token);
    user.set(data.user);
    return data;
}

export function logout() {
    token.set(null);
    user.set(null);
}

export async function checkAuth() {
    const currentToken = getStoredToken();
    if (!currentToken) {
        return false;
    }

    try {
        const response = await fetch('/api/auth/me', {
            headers: { 'Authorization': `Bearer ${currentToken}` }
        });

        if (response.ok) {
            const userData = await response.json();
            user.set(userData);
            return true;
        } else {
            logout();
            return false;
        }
    } catch {
        logout();
        return false;
    }
}

// Helper to get current token synchronously
export function getToken() {
    return getStoredToken();
}
