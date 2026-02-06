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
        // Try to parse error message from response body
        try {
            const errorData = await response.json();
            if (errorData.message) {
                throw new Error(errorData.message);
            }
        } catch (e) {
            if (e.message && !e.message.includes('API error')) {
                throw e;
            }
        }
        throw new Error(`API error: ${response.status}`);
    }

    // Handle 204 No Content (no body to parse)
    if (response.status === 204) {
        return null;
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

export async function updateServer(id, data) {
    return fetchApi(`/servers/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data)
    });
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
