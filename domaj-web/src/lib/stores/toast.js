import { writable } from 'svelte/store';

function createToastStore() {
    const { subscribe, update } = writable([]);

    let id = 0;

    return {
        subscribe,
        success: (message) => {
            const toastId = ++id;
            update(toasts => [...toasts, { id: toastId, message, type: 'success' }]);
            setTimeout(() => {
                update(toasts => toasts.filter(t => t.id !== toastId));
            }, 4000);
        },
        error: (message) => {
            const toastId = ++id;
            update(toasts => [...toasts, { id: toastId, message, type: 'error' }]);
            setTimeout(() => {
                update(toasts => toasts.filter(t => t.id !== toastId));
            }, 5000);
        },
        info: (message) => {
            const toastId = ++id;
            update(toasts => [...toasts, { id: toastId, message, type: 'info' }]);
            setTimeout(() => {
                update(toasts => toasts.filter(t => t.id !== toastId));
            }, 4000);
        },
        dismiss: (toastId) => {
            update(toasts => toasts.filter(t => t.id !== toastId));
        }
    };
}

export const toasts = createToastStore();
