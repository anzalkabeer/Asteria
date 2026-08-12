function sendWindowControl(action) {
    if (window.__TAURI__ && window.__TAURI__.core) {
        window.__TAURI__.core.invoke('asteria_window_control', { action: action });
    } else if (window.__TAURI_INTERNALS__) {
        window.__TAURI_INTERNALS__.invoke('asteria_window_control', { action: action });
    }
}

function showHudToast(message, isAlert = false) {
    let toast = document.getElementById('hud-toast');
    if (!toast) {
        toast = document.createElement('div');
        toast.id = 'hud-toast';
        toast.className = 'fixed bottom-12 right-6 z-50 px-4 py-2 bg-lab-surface border border-lab-border text-xs font-tech rounded shadow-lg transition-all duration-300 transform translate-y-2 opacity-0';
        document.body.appendChild(toast);
    }
    
    toast.textContent = message;
    if (isAlert) {
        toast.className = 'fixed bottom-12 right-6 z-50 px-4 py-2 bg-red-950 border border-red-800 text-red-300 text-xs font-tech rounded shadow-lg transition-all duration-300 transform translate-y-0 opacity-100';
    } else {
        toast.className = 'fixed bottom-12 right-6 z-50 px-4 py-2 bg-lab-surface border border-lab-accent text-lab-accent text-xs font-tech rounded shadow-lg transition-all duration-300 transform translate-y-0 opacity-100';
    }

    setTimeout(() => {
        toast.className = 'fixed bottom-12 right-6 z-50 px-4 py-2 bg-lab-surface border border-lab-border text-xs font-tech rounded shadow-lg transition-all duration-300 transform translate-y-2 opacity-0';
    }, 4000);
}

function applyTheme(theme) {
    const themeBtn = document.getElementById('theme-toggle-btn');
    const body = document.body;
    
    if (theme === 'light') {
        body.style.backgroundColor = '#ffffff';
        body.style.color = '#0f172a';
        if (themeBtn) themeBtn.textContent = 'DARK MODE';
        showHudToast('[THEME] Laboratory UI switched to Light Mode');
    } else {
        body.style.backgroundColor = '#0b1326';
        body.style.color = '#f8fafc';
        if (themeBtn) themeBtn.textContent = 'LIGHT MODE';
        showHudToast('[THEME] Laboratory UI switched to Dark Mode');
    }
}

function toggleThemeMode() {
    let currentTheme = document.body.dataset.theme || 'dark';
    let newTheme = currentTheme === 'dark' ? 'light' : 'dark';
    document.body.dataset.theme = newTheme;

    if (window.__TAURI__ && window.__TAURI__.core) {
        window.__TAURI__.core.invoke('asteria_toggle_theme').then(() => {
            applyTheme(newTheme);
        }).catch(() => {
            applyTheme(newTheme);
        });
    } else {
        applyTheme(newTheme);
    }
}

function executeDiagnosticCommand(cmd) {
    if (!cmd.trim()) return;

    if (window.__TAURI__ && window.__TAURI__.core) {
        window.__TAURI__.core.invoke('asteria_execute_command', { command: cmd })
            .then(res => {
                const trimmed = cmd.trim().toLowerCase();
                if (trimmed === 'theme' || trimmed === 'mode') {
                    toggleThemeMode();
                } else if (res && res.message) {
                    showHudToast(res.message);
                }
            })
            .catch(err => {
                showHudToast(`[ERROR] ${err}`, true);
            });
    } else {
        const trimmed = cmd.trim().toLowerCase();
        if (trimmed === 'theme' || trimmed === 'mode') {
            toggleThemeMode();
        } else {
            showHudToast(`[LOCAL] Executed diagnostic query: ${cmd}`);
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    // 1. Window Control Buttons
    const minBtn = document.getElementById('win-min');
    const maxBtn = document.getElementById('win-max');
    const closeBtn = document.getElementById('win-close');

    if (minBtn) {
        minBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            sendWindowControl('minimize');
        });
    }

    if (maxBtn) {
        maxBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            sendWindowControl('maximize');
        });
    }

    if (closeBtn) {
        closeBtn.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            sendWindowControl('close');
        });
    }

    // 2. Theme Toggle Link/Button
    const themeBtn = document.getElementById('theme-toggle-btn');
    if (themeBtn) {
        themeBtn.addEventListener('click', (e) => {
            e.preventDefault();
            toggleThemeMode();
        });
    }

    // 3. Command Line Input Box Execution
    const cmdInput = document.querySelector('input[type="text"]');
    if (cmdInput) {
        cmdInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const val = cmdInput.value;
                cmdInput.value = '';
                executeDiagnosticCommand(val);
            }
        });
    }

    // 4. Tab Switching
    const tabs = document.querySelectorAll('.nav-tab');
    tabs.forEach(tab => {
        tab.addEventListener('click', (e) => {
            tabs.forEach(t => {
                t.classList.remove('border-t-2', 'border-t-lab-accent', 'bg-lab-white', 'text-lab-accent', 'font-semibold', 'h-10');
                t.classList.add('bg-lab-surface', 'text-lab-muted', 'font-medium', 'h-9');
            });
            tab.classList.remove('bg-lab-surface', 'text-lab-muted', 'font-medium', 'h-9');
            tab.classList.add('border-t-2', 'border-t-lab-accent', 'bg-lab-white', 'text-lab-accent', 'font-semibold', 'h-10');
            showHudToast(`[TAB] Switched active view to ${tab.textContent.trim().toUpperCase()}`);
        });
    });

    // 5. Command Line Shortcut (Ctrl+K or Cmd+K)
    document.addEventListener('keydown', (e) => {
        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
            e.preventDefault();
            if (cmdInput) cmdInput.focus();
        }
    });
});
