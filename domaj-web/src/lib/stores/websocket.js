import { writable } from 'svelte/store';
import { toasts } from './toast.js';
import { getToken } from './auth.js';

function createWebSocketStore() {
    const { subscribe, set } = writable(null);
    let ws = null;
    let reconnectTimer = null;
    let lastEvent = writable(null);

    function connect() {
        const token = getToken();
        if (!token) return;

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const url = `${protocol}//${window.location.host}/api/ws?token=${encodeURIComponent(token)}`;

        ws = new WebSocket(url);

        ws.onopen = () => {
            set('connected');
        };

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                lastEvent.set(data);

                if (data.type === 'job_started') {
                    toasts.info(`🔄 Mise à jour démarrée : ${data.job.container_name}`);
                } else if (data.type === 'job_completed') {
                    toasts.success(`✅ Mise à jour réussie : ${data.job.container_name}`);
                } else if (data.type === 'job_failed') {
                    toasts.error(`❌ Échec mise à jour : ${data.job.container_name}`);
                }
            } catch (e) {
                // ignore parse errors
            }
        };

        ws.onclose = () => {
            set('disconnected');
            // Reconnect after 5 seconds
            reconnectTimer = setTimeout(connect, 5000);
        };

        ws.onerror = () => {
            ws.close();
        };
    }

    function disconnect() {
        if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
        }
        if (ws) {
            ws.close();
            ws = null;
        }
        set(null);
    }

    return {
        subscribe,
        lastEvent,
        connect,
        disconnect
    };
}

export const websocketStore = createWebSocketStore();
