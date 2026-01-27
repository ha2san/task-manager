// =========================================================
// CONFIGURATION
// =========================================================
const API_URL = '/api';
let currentChart = null;

// =========================================================
// GESTION DES REQUÊTES API
// =========================================================
async function apiFetch(path, options = {}) {
    let token;
    try {
        token = localStorage.getItem('token');
    } catch (err) {
        showNotification("Impossible d'accéder au stockage local. Désactivez le mode privé sur iOS.", 'error');
        return null;
    }

    if (!token && !path.includes('/auth/')) {
        window.location.href = 'auth.html';
        return null;
    }

    const config = {
        ...options,
        headers: {
            'Content-Type': 'application/json',
            ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
            ...options.headers
        }
    };

    try {
        const response = await fetch(`${API_URL}${path}`, config);

        if (response.status === 401) {
            localStorage.removeItem('token');
            window.location.href = 'auth.html';
            return null;
        }

        if (!response.ok) {
            const errorText = await response.text();
            console.error(`API error ${response.status}:`, errorText);
            
            if (response.status !== 204) { // 204 No Content n'a pas de corps
                try {
                    const errorData = JSON.parse(errorText);
                    showNotification(errorData.message || `Erreur ${response.status}`, 'error');
                } catch {
                    showNotification(`Erreur ${response.status}`, 'error');
                }
            }
            
            return null;
        }

        if (response.status === 204) return true;

        const contentType = response.headers.get('content-type');
        if (contentType && contentType.includes('application/json')) {
            return response.json();
        }

        return true;
    } catch (err) {
        console.error("Erreur réseau :", err);
        showNotification("Erreur de connexion. Vérifiez votre réseau.", 'error');
        return null;
    }
}

// =========================================================
// GESTION DES TÂCHES
// =========================================================
async function fetchTasks() {
    const loading = document.getElementById('tasks-loading');
    const empty = document.getElementById('tasks-empty');
    const list = document.getElementById('tasks-list');
    
    if (loading) loading.style.display = 'flex';
    if (empty) empty.style.display = 'none';
    if (list) list.innerHTML = '';
    
    try {
        const tasks = await apiFetch('/tasks');
        if (!tasks) return;
        
        if (loading) loading.style.display = 'none';
        
        if (tasks.length === 0) {
            if (empty) empty.style.display = 'flex';
            if (list) list.innerHTML = '';
            updateTaskCounters();
            return;
        }
        
        // Trier par priorité
        tasks.sort((a, b) => (a.priority || 0) - (b.priority || 0));
        
        // Afficher les tâches
        renderTasks(tasks);
        
        // Mettre à jour les compteurs
        updateTaskCounters(tasks);
        
        // Initialiser le drag and drop
        initTaskSorting();
        
    } catch (error) {
        console.error('Erreur lors du chargement des tâches:', error);
        if (loading) loading.style.display = 'none';
    }
}

function renderTasks(tasks) {
    const list = document.getElementById('tasks-list');
    if (!list) return;
    
    list.innerHTML = tasks.map(task => `
        <li class="task-item ${task.completed ? 'completed' : ''} ${task.has_subtasks ? 'has-subtasks' : ''}" 
            data-id="${task.id}" data-priority="${task.priority || 0}">
            <div class="task-main">
                <div class="task-header">
                    <div class="task-checkbox">
                        <input type="checkbox" 
                               id="task-${task.id}" 
                               ${task.completed ? 'checked' : ''}
                               onchange="toggleTask(${task.id})"
                               ${task.has_subtasks ? 'data-has-subtasks="true"' : ''}>
                        <label for="task-${task.id}" class="checkbox-custom"></label>
                    </div>
                    <div class="task-title" onclick="toggleTask(${task.id})">
                        ${escapeHtml(task.title)}
                        ${task.has_subtasks ? 
                            `<span class="subtask-indicator">
                                <i class="fas fa-list-check"></i>
                                ${task.subtasks ? task.subtasks.length : 0}
                            </span>` : ''}
                    </div>
                    <div class="task-actions">
                        <button class="btn-icon" onclick="editTaskFromDashboard(${task.id})" title="Modifier">
                            <i class="fas fa-edit"></i>
                        </button>
                        <span class="task-priority">#${task.priority + 1}</span>
                    </div>
                </div>
                
                ${task.has_subtasks && task.subtasks && task.subtasks.length > 0 ? `
                <div class="subtasks-container">
                    ${task.subtasks.map(subtask => `
                        <div class="subtask-item ${subtask.completed ? 'completed' : ''}" 
                             data-id="${task.id}" data-subtask-id="${subtask.id}">
                            <div class="subtask-checkbox">
                                <input type="checkbox" 
                                       id="subtask-${subtask.id}" 
                                       ${subtask.completed ? 'checked' : ''}
                                       onchange="toggleSubtask(${task.id}, ${subtask.id})">
                                <label for="subtask-${subtask.id}" class="checkbox-custom small"></label>
                            </div>
                            <span class="subtask-title" onclick="toggleSubtask(${task.id}, ${subtask.id})">
                                ${escapeHtml(subtask.title)}
                            </span>
                            <div class="subtask-priority">#${subtask.priority + 1}</div>
                        </div>
                    `).join('')}
                </div>
                ` : ''}
                
                <div class="add-subtask-form">
                    <input type="text" 
                           class="subtask-input" 
                           placeholder="Ajouter une sous-tâche..." 
                           data-task-id="${task.id}"
                           onkeypress="if(event.key === 'Enter') addSubtask(${task.id}, this)">
                    <button class="btn-icon" onclick="addSubtask(${task.id}, this.previousElementSibling)">
                        <i class="fas fa-plus"></i>
                    </button>
                </div>
            </div>
        </li>
    `).join('');
}

function updateTaskCounters(tasks = []) {
    const total = tasks.length;
    const completed = tasks.filter(t => t.completed).length;
    const pending = total - completed;
    const withSubtasks = tasks.filter(t => t.has_subtasks).length;
    const progress = total > 0 ? Math.round((completed / total) * 100) : 0;
    
    // Mettre à jour l'interface
    const countAll = document.getElementById('count-all');
    const countPending = document.getElementById('count-pending');
    const countCompleted = document.getElementById('count-completed');
    const countSubtasks = document.getElementById('count-subtasks');
    const totalTasksCount = document.getElementById('total-tasks-count');
    const doneTasksCount = document.getElementById('done-tasks-count');
    const progressPercent = document.getElementById('progress-percent');
    const footerTotal = document.getElementById('footer-total');
    const footerCompleted = document.getElementById('footer-completed');
    
    if (countAll) countAll.textContent = total;
    if (countPending) countPending.textContent = pending;
    if (countCompleted) countCompleted.textContent = completed;
    if (countSubtasks) countSubtasks.textContent = withSubtasks;
    if (totalTasksCount) totalTasksCount.textContent = total;
    if (doneTasksCount) doneTasksCount.textContent = completed;
    if (progressPercent) progressPercent.textContent = `${progress}%`;
    if (footerTotal) footerTotal.textContent = total;
    if (footerCompleted) footerCompleted.textContent = completed;
    
    // Mettre à jour la barre de progression
    updateProgressBars(progress);
}

function initTaskSorting() {
    const list = document.getElementById('tasks-list');
    if (!list || window.innerWidth < 768) return;
    
    try {
        Sortable.create(list, {
            animation: 150,
            ghostClass: 'sortable-ghost',
            dragClass: 'sortable-drag',
            handle: '.task-title',
            onEnd: async function() {
                const orderedIds = Array.from(list.children)
                    .map(li => parseInt(li.dataset.id))
                    .filter(id => !isNaN(id));
                
                if (orderedIds.length > 0) {
                    await updateTaskPriorities(orderedIds);
                }
            }
        });
    } catch (error) {
        console.error('Erreur lors de l\'initialisation du tri:', error);
    }
}

async function updateTaskPriorities(orderedIds) {
    try {
        await apiFetch('/tasks/priorities', {
            method: 'POST',
            body: JSON.stringify({ ordered_task_ids: orderedIds })
        });
        
        // Mettre à jour l'affichage des priorités
        document.querySelectorAll('.task-item').forEach((item, index) => {
            const prioritySpan = item.querySelector('.task-priority');
            if (prioritySpan) {
                prioritySpan.textContent = `#${index + 1}`;
            }
        });
        
        showNotification('Priorités mises à jour', 'success');
    } catch (error) {
        showNotification('Erreur lors de la mise à jour des priorités', 'error');
    }
}

async function toggleTask(taskId) {
    try {
        await apiFetch(`/tasks/${taskId}/toggle`, { method: 'POST' });
        await fetchTasks();
        await fetchStats(); // Rafraîchir les statistiques
    } catch (error) {
        showNotification('Erreur lors de la modification de la tâche', 'error');
    }
}

async function toggleSubtask(taskId, subtaskId) {
    try {
        await apiFetch('/subtasks/toggle', {
            method: 'POST',
            body: JSON.stringify({ task_id: taskId, subtask_id: subtaskId })
        });
        await fetchTasks();
        await fetchStats();
    } catch (error) {
        showNotification('Erreur lors de la modification de la sous-tâche', 'error');
    }
}

async function addSubtask(taskId, inputElement) {
    const title = inputElement.value.trim();
    if (!title) return;
    
    try {
        const success = await apiFetch(`/tasks/${taskId}/subtasks`, {
            method: 'POST',
            body: JSON.stringify({ title })
        });
        
        if (success) {
            inputElement.value = '';
            await fetchTasks();
            showNotification('Sous-tâche ajoutée', 'success');
        }
    } catch (error) {
        showNotification('Erreur lors de l\'ajout de la sous-tâche', 'error');
    }
}

function editTaskFromDashboard(taskId) {
    // Rediriger vers la page de gestion avec l'ID de la tâche
    localStorage.setItem('editTaskId', taskId);
    window.location.href = 'manage.html';
}

// =========================================================
// GESTION DES STATISTIQUES
// =========================================================
async function fetchStats() {
    try {
        const stats = await apiFetch('/stats');
        if (!stats) return;
        
        updateStatsDisplay(stats);
        renderHeatmap(stats.history);
        initProgressChart(stats.history);
        
    } catch (error) {
        console.error('Erreur lors du chargement des statistiques:', error);
    }
}

function updateStatsDisplay(stats) {
    const successRate = document.getElementById('success-rate');
    const todayRate = document.getElementById('today-rate');
    const currentStreak = document.getElementById('current-streak');
    const avgProductivity = document.getElementById('avg-productivity');
    const footerStreak = document.getElementById('footer-streak');
    
    if (successRate) {
        successRate.textContent = `${stats.summary.success_rate || 0}%`;
    }
    
    if (todayRate) {
        todayRate.textContent = `${stats.summary.today_percent || 0}%`;
    }
    
    if (currentStreak) {
        // Calculer la série actuelle (simplifié)
        const streak = calculateCurrentStreak(stats.history);
        currentStreak.textContent = `${streak} jour${streak > 1 ? 's' : ''}`;
    }
    
    if (avgProductivity) {
        const avg = calculateAverageProductivity(stats.history);
        avgProductivity.textContent = `${avg}%`;
    }
    
    if (footerStreak) {
        const streak = calculateCurrentStreak(stats.history);
        footerStreak.textContent = streak;
    }
    
    // Mettre à jour les tendances
    updateTrends(stats);
}

function calculateCurrentStreak(history) {
    if (!history || history.length === 0) return 0;
    
    let streak = 0;
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    
    // Trier par date décroissante
    const sortedHistory = [...history].sort((a, b) => 
        new Date(b.date) - new Date(a.date)
    );
    
    for (const day of sortedHistory) {
        const dayDate = new Date(day.date);
        if (day.percent > 0) {
            streak++;
        } else {
            break;
        }
    }
    
    return streak;
}

function calculateAverageProductivity(history) {
    if (!history || history.length === 0) return 0;
    
    const validDays = history.filter(day => day.percent !== null);
    if (validDays.length === 0) return 0;
    
    const sum = validDays.reduce((total, day) => total + day.percent, 0);
    return Math.round(sum / validDays.length);
}

function updateTrends(stats) {
    // Implémenter la logique de calcul des tendances
    // Pour l'instant, valeurs simulées
    document.getElementById('success-trend').innerHTML = 
        `<i class="fas fa-arrow-up"></i><span>2.5%</span>`;
    document.getElementById('today-trend').innerHTML = 
        `<i class="fas fa-minus"></i><span>0%</span>`;
    document.getElementById('streak-trend').innerHTML = 
        `<i class="fas fa-arrow-up"></i><span>Record: 14</span>`;
    document.getElementById('productivity-trend').innerHTML = 
        `<i class="fas fa-arrow-up"></i><span>1.2%</span>`;
}

function renderHeatmap(history) {
    const container = document.getElementById('heatmap-container');
    if (!container) return;
    
    container.innerHTML = '';
    
    // Trier par date croissante
    const sortedHistory = [...history].sort((a, b) => 
        new Date(a.date) - new Date(b.date)
    );
    
    sortedHistory.forEach(day => {
        const square = document.createElement('div');
        square.className = 'heatmap-square';
        square.title = `${formatDate(day.date)}: ${day.percent}%`;
        
        // Déterminer la couleur en fonction du pourcentage
        let color = '#ebedf0'; // 0%
        if (day.percent > 0) color = '#9be9a8'; // 1-25%
        if (day.percent > 25) color = '#40c463'; // 26-50%
        if (day.percent > 50) color = '#30a14e'; // 51-75%
        if (day.percent > 75) color = '#216e39'; // 76-100%
        
        square.style.backgroundColor = color;
        
        // Tooltip
        square.addEventListener('mouseenter', (e) => {
            showHeatmapTooltip(e, day);
        });
        
        square.addEventListener('mouseleave', () => {
            hideHeatmapTooltip();
        });
        
        container.appendChild(square);
    });
}

function showHeatmapTooltip(event, day) {
    const tooltip = document.getElementById('heatmap-tooltip');
    if (!tooltip) return;
    
    const tooltipDate = document.getElementById('tooltip-date');
    const tooltipCompleted = document.getElementById('tooltip-completed');
    const tooltipScheduled = document.getElementById('tooltip-scheduled');
    const tooltipPercent = document.getElementById('tooltip-percent');
    
    if (tooltipDate) tooltipDate.textContent = formatDate(day.date, true);
    if (tooltipCompleted) tooltipCompleted.textContent = Math.round(day.completed || 0);
    if (tooltipScheduled) tooltipScheduled.textContent = Math.round(day.scheduled || 0);
    if (tooltipPercent) tooltipPercent.textContent = `${day.percent}%`;
    
    tooltip.style.display = 'block';
    tooltip.style.left = `${event.pageX + 10}px`;
    tooltip.style.top = `${event.pageY - 10}px`;
}

function hideHeatmapTooltip() {
    const tooltip = document.getElementById('heatmap-tooltip');
    if (tooltip) {
        tooltip.style.display = 'none';
    }
}

function initProgressChart(history) {
    const ctx = document.getElementById('progress-chart');
    if (!ctx || !window.Chart) return;
    
    // Détruire le graphique existant
    if (currentChart) {
        currentChart.destroy();
    }
    
    // Préparer les données des 7 derniers jours
    const last7Days = [...history].slice(-7);
    const labels = last7Days.map(day => formatDate(day.date, true));
    const data = last7Days.map(day => day.percent);
    
    currentChart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: labels,
            datasets: [{
                label: 'Progression',
                data: data,
                borderColor: '#6366f1',
                backgroundColor: 'rgba(99, 102, 241, 0.1)',
                borderWidth: 2,
                fill: true,
                tension: 0.3
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    display: false
                },
                tooltip: {
                    mode: 'index',
                    intersect: false
                }
            },
            scales: {
                y: {
                    beginAtZero: true,
                    max: 100,
                    grid: {
                        color: 'rgba(0, 0, 0, 0.05)'
                    },
                    ticks: {
                        callback: function(value) {
                            return value + '%';
                        }
                    }
                },
                x: {
                    grid: {
                        display: false
                    }
                }
            }
        }
    });
    
    // Mettre à jour les résumés
    updateChartSummary(last7Days);
}

function updateChartSummary(last7Days) {
    if (!last7Days || last7Days.length === 0) return;
    
    // Meilleur jour
    const bestDay = last7Days.reduce((best, day) => 
        day.percent > best.percent ? day : best
    );
    
    // Moyenne
    const avg = last7Days.reduce((sum, day) => sum + day.percent, 0) / last7Days.length;
    
    // Tendance (comparaison premier/dernier jour)
    const firstDay = last7Days[0].percent;
    const lastDay = last7Days[last7Days.length - 1].percent;
    const trend = lastDay - firstDay;
    
    const bestDayEl = document.getElementById('best-day');
    const avgCompletionEl = document.getElementById('avg-completion');
    const completionTrendEl = document.getElementById('completion-trend');
    
    if (bestDayEl) {
        bestDayEl.textContent = formatDate(bestDay.date, false);
    }
    
    if (avgCompletionEl) {
        avgCompletionEl.textContent = `${Math.round(avg)}%`;
    }
    
    if (completionTrendEl) {
        completionTrendEl.textContent = trend > 0 ? '↑ Amélioration' : 
                                      trend < 0 ? '↓ Baisse' : '→ Stable';
        completionTrendEl.className = `summary-value ${
            trend > 0 ? 'trend-up' : trend < 0 ? 'trend-down' : ''
        }`;
    }
}

function updateProgressBars(progress) {
    const weeklyProgress = document.getElementById('weekly-progress');
    const weeklyProductivity = document.getElementById('weekly-productivity');
    const monthlyProgress = document.getElementById('monthly-progress');
    const monthlyEngagement = document.getElementById('monthly-engagement');
    
    if (weeklyProgress) {
        weeklyProgress.style.width = `${progress}%`;
    }
    
    if (weeklyProductivity) {
        weeklyProductivity.textContent = `${progress}%`;
    }
    
    // Simuler des données pour la barre mensuelle
    const monthlyProgressValue = Math.min(100, progress + 15);
    if (monthlyProgress) {
        monthlyProgress.style.width = `${monthlyProgressValue}%`;
    }
    
    if (monthlyEngagement) {
        monthlyEngagement.textContent = `${monthlyProgressValue}%`;
    }
}


// =========================================================
// UTILITAIRES
// =========================================================
function formatDate(dateString, full = false) {
    const date = new Date(dateString);
    const today = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    
    if (date.toDateString() === today.toDateString()) {
        return 'Aujourd\'hui';
    } else if (date.toDateString() === yesterday.toDateString()) {
        return 'Hier';
    } else if (full) {
        return date.toLocaleDateString('fr-FR', { 
            weekday: 'short', 
            day: 'numeric', 
            month: 'short' 
        });
    } else {
        return date.toLocaleDateString('fr-FR', { 
            day: 'numeric', 
            month: 'short' 
        });
    }
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function showNotification(message, type = 'info') {
    // Créer une notification
    const container = document.getElementById('notification-container') || createNotificationContainer();
    
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.innerHTML = `
        <i class="fas fa-${type === 'success' ? 'check-circle' : 
                          type === 'error' ? 'exclamation-circle' : 
                          'info-circle'}"></i>
        <span>${message}</span>
        <button class="notification-close" onclick="this.parentElement.remove()">&times;</button>
    `;
    
    container.appendChild(notification);
    
    // Animation
    setTimeout(() => notification.classList.add('show'), 10);
    
    // Auto-remove après 5 secondes
    setTimeout(() => {
        if (notification.parentNode) {
            notification.classList.remove('show');
            setTimeout(() => notification.remove(), 300);
        }
    }, 5000);
}

function createNotificationContainer() {
    const container = document.createElement('div');
    container.id = 'notification-container';
    container.className = 'notification-container';
    document.body.appendChild(container);
    return container;
}

// =========================================================
// GESTION DE LA DÉCONNEXION
// =========================================================
function logout() {
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    window.location.href = 'auth.html';
}

// =========================================================
// INITIALISATION
// =========================================================
document.addEventListener('DOMContentLoaded', function() {
    // Vérifier l'authentification
    const token = localStorage.getItem('token');
    if (!token && !window.location.pathname.includes('auth.html')) {
        window.location.href = 'auth.html';
        return;
    }
    
    // Récupérer le nom d'utilisateur si disponible
    const user = localStorage.getItem('user');
    if (user) {
        try {
            const userData = JSON.parse(user);
            const usernameDisplay = document.getElementById('username-display');
            const dropdownUsername = document.getElementById('dropdown-username');
            
            if (usernameDisplay) usernameDisplay.textContent = userData.username;
            if (dropdownUsername) dropdownUsername.textContent = userData.username;
        } catch (e) {
            console.error('Erreur lors du parsing des données utilisateur:', e);
        }
    }
    
    // Initialiser les composants spécifiques à la page
    if (document.getElementById('tasks-list')) {
        fetchTasks();
        fetchStats();
    }
    
    // Initialiser les tooltips de la heatmap
    document.addEventListener('mousemove', function(e) {
        const tooltip = document.getElementById('heatmap-tooltip');
        if (tooltip && tooltip.style.display === 'block') {
            tooltip.style.left = `${e.pageX + 10}px`;
            tooltip.style.top = `${e.pageY - 10}px`;
        }
    });
});

// =========================================================
// FONCTIONS GLOBALES POUR LES AUTRES PAGES
// =========================================================
window.showNotification = showNotification;
window.apiFetch = apiFetch;
window.logout = logout;
window.fetchTasks = fetchTasks;
window.fetchStats = fetchStats;

// Exporter pour utilisation dans les autres fichiers
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        apiFetch,
        showNotification,
        logout,
        fetchTasks,
        fetchStats
    };
}
