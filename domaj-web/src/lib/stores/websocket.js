import { writable } from 'svelte/store';
import { toasts } from './toast.js';
import { getToken } from './auth.js';

function createWebSocketStore() {
    const { subscribe, set } = writable(null);
    let ws = null;
    let reconnectTimer = null;
    let lastEvent = writable(null);
    let refCount = 0;

    function connect() {
        refCount++;
        // Don't create a new connection if one is already open or connecting
        if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
            return;
        }

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
                    const label = data.job.job_type === 'rollback' ? 'Rollback' : data.job.job_type === 'auto_rollback' ? 'Restauration auto' : 'Mise à jour';
                    toasts.info(`🔄 ${label} démarré : ${data.job.container_name}`);
                } else if (data.type === 'job_completed') {
                    const label = data.job.job_type === 'rollback' ? 'Rollback réussi' : data.job.job_type === 'auto_rollback' ? 'Restauration auto réussie' : 'Mise à jour réussie';
                    toasts.success(`✅ ${label} : ${data.job.container_name}`);
                } else if (data.type === 'job_failed') {
                    const label = data.job.job_type === 'rollback' ? 'Échec du rollback' : data.job.job_type === 'auto_rollback' ? 'Échec de la restauration auto' : 'Échec de la mise à jour';
                    toasts.error(`❌ ${label} : ${data.job.container_name}`);
                }
            } catch (e) {
                // ignore parse errors
            }
        };

        ws.onclose = () => {
            set('disconnected');
            // Reconnect after 5 seconds only if still referenced
            if (refCount > 0) {
                reconnectTimer = setTimeout(connect, 5000);
            }
        };

        ws.onerror = () => {
            ws.close();
        };
    }

    function disconnect() {
        refCount = Math.max(0, refCount - 1);
        if (refCount > 0) return; // Other pages still need the connection

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
