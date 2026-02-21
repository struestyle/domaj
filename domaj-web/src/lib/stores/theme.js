import { writable } from 'svelte/store';

const STORAGE_KEY = 'domaj-theme';

function getInitialTheme() {
    if (typeof window === 'undefined') return 'dark';
    return localStorage.getItem(STORAGE_KEY) || 'dark';
}

export const theme = writable(getInitialTheme());

export function toggleTheme() {
    theme.update(current => {
        const next = current === 'dark' ? 'light' : 'dark';
        localStorage.setItem(STORAGE_KEY, next);
        document.documentElement.setAttribute('data-theme', next);
        return next;
    });
}

export function initTheme() {
    const saved = getInitialTheme();
    document.documentElement.setAttribute('data-theme', saved);
    theme.set(saved);
}
