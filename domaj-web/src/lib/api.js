const API_BASE = '/api';

export async function fetchApi(endpoint, options = {}) {
    const url = `${API_BASE}${endpoint}`;
    const response = await fetch(url, {
        headers: {
            'Content-Type': 'application/json',
            ...options.headers
        },
        ...options
    });

    if (!response.ok) {
        throw new Error(`API error: ${response.status}`);
    }

    return response.json();
}

export async function getServers() {
    return fetchApi('/servers');
}

export async function getServer(id) {
    return fetchApi(`/servers/${id}`);
}

export async function createServer(data) {
    return fetchApi('/servers', {
        method: 'POST',
        body: JSON.stringify(data)
    });
}

export async function deleteServer(id) {
    return fetchApi(`/servers/${id}`, { method: 'DELETE' });
}

export async function syncServer(id) {
    return fetchApi(`/servers/${id}/sync`, { method: 'POST' });
}

export async function getServerContainers(id) {
    return fetchApi(`/servers/${id}/containers`);
}

export async function getContainers() {
    return fetchApi('/containers');
}

export async function getUpdates() {
    return fetchApi('/updates');
}

export async function triggerScan() {
    return fetchApi('/scan', { method: 'POST' });
}

export async function getStatus() {
    return fetchApi('/status');
}
