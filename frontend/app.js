const API_URL = '/api';

/* =========================================================
   API FETCH CENTRALISÉ (OBLIGATOIRE)
========================================================= */
async function apiFetch(path, options = {}) {
    const token = localStorage.getItem('token');
    if (!token) {
        window.location.href = 'auth.html';
        return null;
    }

    const config = {
        ...options,
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${token}`,
            ...options.headers
        }
    };

    const response = await fetch(`${API_URL}${path}`, config);

    if (response.status === 401) {
        localStorage.removeItem('token');
        window.location.href = 'auth.html';
        return null;
    }

    if (!response.ok) {
        console.error(`API error ${response.status}`);
        return null;
    }

    if (response.status === 204) return true;

    const contentType = response.headers.get('content-type');
    if (contentType && contentType.includes('application/json')) {
        return response.json();
    }

    return true;
}

/* =========================================================
   DASHBOARD (index.html)
========================================================= */
async function fetchTasks() {
    const list = document.getElementById('tasks');
    if (!list) return;

    const tasks = await apiFetch('/tasks');
    if (!tasks) return;

    list.innerHTML = tasks.map(task => `
        <li class="task-item ${task.completed ? 'completed' : ''}">
            <span onclick="toggleTask(${task.id})">${task.title}</span>
            <input type="checkbox"
                   ${task.completed ? 'checked' : ''}
                   onchange="toggleTask(${task.id})">
        </li>
    `).join('');
}

async function toggleTask(id) {
    await apiFetch(`/tasks/${id}/toggle`, { method: 'POST' });
    fetchTasks();
    fetchStats();
}

/* ================= STATS ================= */

async function fetchStats() {
    const data = await apiFetch('/stats');
    if (!data) return;

    renderHeatmap(data.history);

    document.getElementById('stat-total-created').textContent =
        data.summary.total_created;

    document.getElementById('stat-total-done').textContent =
        data.summary.total_completed_ever;
}

function renderHeatmap(history) {
    const container = document.getElementById('heatmap-container');
    if (!container) return;
    container.innerHTML = '';

    history.forEach(day => {
        const square = document.createElement('div');
        square.className = 'heatmap-square';

        let color = '#ebedf0';
        if (day.percent > 0) color = '#9be9a8';
        if (day.percent > 40) color = '#40c463';
        if (day.percent > 80) color = '#216e39';

        square.style.backgroundColor = color;
        container.appendChild(square);
    });
}

/* =========================================================
   MANAGE PAGE
========================================================= */
let allTasksCache = [];

/* -------- IMPORT JSON -------- */
document.getElementById('importFile')?.addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;

    try {
        const tasks = JSON.parse(await file.text());
        if (!Array.isArray(tasks)) throw new Error();

        for (const task of tasks) {
            if (!task.title || !Array.isArray(task.days)) continue;
            await apiFetch('/tasks', {
                method: 'POST',
                body: JSON.stringify(task)
            });
        }

        alert("Import terminé ✅");
        fetchAllTasks();
    } catch {
        alert("Fichier JSON invalide");
    }
});

/* -------- FETCH & FILTRES -------- */
async function fetchAllTasks() {
    const tasks = await apiFetch('/tasks/all');
    if (!tasks) return;
    allTasksCache = tasks;
    renderFilteredTasks();
}

function renderFilteredTasks() {
    const container = document.getElementById('all-tasks');
    if (!container) return;

    const search = document.getElementById('searchInput')?.value.toLowerCase() || '';
    const selectedDays = Array.from(
        document.querySelectorAll('.days-filter input:checked')
    ).map(cb => parseInt(cb.value));

    const filtered = allTasksCache.filter(task => {
        const matchText = task.title.toLowerCase().includes(search);
        const matchDays =
            selectedDays.length === 0 ||
            selectedDays.some(d => task.days.includes(d));
        return matchText && matchDays;
    });

    container.innerHTML = filtered.map(renderTaskCard).join('');
}

function renderTaskCard(task) {
    return `
    <div class="manage-task-card ${!task.active ? 'archived' : ''}">
        <input class="edit-title"
               id="title-${task.id}"
               value="${task.title}"
               onchange="updateTask(${task.id})">

        <div class="edit-days">
            ${[1,2,3,4,5,6,7].map(d => `
                <label>
                    <input type="checkbox"
                           data-task="${task.id}"
                           value="${d}"
                           ${task.days.includes(d) ? 'checked' : ''}
                           onchange="updateTask(${task.id})">
                    ${['L','M','M','J','V','S','D'][d-1]}
                </label>
            `).join('')}
        </div>

        <div class="manage-actions">
            <button class="btn-secondary" onclick="toggleArchive(${task.id})">
                ${task.active ? 'Archiver' : 'Activer'}
            </button>
            <button class="btn-danger" onclick="deleteTask(${task.id})">
                Supprimer
            </button>
        </div>
    </div>
    `;
}

/* -------- ACTIONS -------- */
async function updateTask(id) {
    const title = document.getElementById(`title-${id}`).value;
    const days = Array.from(
        document.querySelectorAll(`input[data-task="${id}"]:checked`)
    ).map(cb => parseInt(cb.value));

    await apiFetch(`/tasks/${id}`, {
        method: 'POST',
        body: JSON.stringify({ title, days })
    });
}

async function toggleArchive(id) {
    await apiFetch(`/tasks/${id}`, { method: 'PATCH' });
    fetchAllTasks();
}

async function deleteTask(id) {
    if (!confirm("Supprimer cette tâche ?")) return;
    await apiFetch(`/tasks/${id}`, { method: 'DELETE' });
    fetchAllTasks();
}

/* -------- CRÉATION -------- */
document.getElementById('task-form')?.addEventListener('submit', async (e) => {
    e.preventDefault();

    const title = document.getElementById('task-title').value;
    const days = Array.from(
        document.querySelectorAll('input[name="days"]:checked')
    ).map(cb => parseInt(cb.value));

    if (!title || !days.length) {
        alert("Titre et jours requis");
        return;
    }

    await apiFetch('/tasks', {
        method: 'POST',
        body: JSON.stringify({ title, days })
    });

    e.target.reset();
    fetchAllTasks();
});

/* -------- LISTENERS FILTRES -------- */
document.getElementById('searchInput')
    ?.addEventListener('input', renderFilteredTasks);

document.querySelectorAll('.days-filter input')
    .forEach(cb => cb.addEventListener('change', renderFilteredTasks));

/* =========================================================
   INITIALISATION GLOBALE
========================================================= */
document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('tasks')) {
        fetchTasks();
        fetchStats();
    }

    if (document.getElementById('all-tasks')) {
        fetchAllTasks();
    }
});

