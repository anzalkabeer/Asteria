// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use asteria::shell::TabManager;
use std::sync::Mutex;
use tauri::{State, Window};

struct AppState {
    tab_manager: Mutex<TabManager>,
    theme_mode: Mutex<String>,
}

#[tauri::command]
fn asteria_window_control(action: String, window: Window) -> Result<(), String> {
    match action.as_str() {
        "minimize" => window.minimize().map_err(|e| e.to_string()),
        "maximize" => {
            let is_max = window.is_maximized().unwrap_or(false);
            if is_max {
                window.unmaximize().map_err(|e| e.to_string())
            } else {
                window.maximize().map_err(|e| e.to_string())
            }
        }
        "close" => window.close().map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}

#[tauri::command]
fn asteria_navigate(url: String, state: State<'_, AppState>) -> Result<String, String> {
    let mut mgr = state.tab_manager.lock().map_err(|e| e.to_string())?;
    mgr.navigate(&url).map_err(|e| e.to_string())?;
    Ok(format!("Navigated to {}", url))
}

#[tauri::command]
fn asteria_toggle_theme(state: State<'_, AppState>) -> Result<String, String> {
    let mut theme = state.theme_mode.lock().map_err(|e| e.to_string())?;
    *theme = if *theme == "dark" {
        "light".to_string()
    } else {
        "dark".to_string()
    };
    Ok(theme.clone())
}

#[tauri::command]
fn asteria_get_telemetry() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "fps": 144,
        "latency_ms": 12,
        "engine_version": "v0.1.0",
        "kernel": "AST-902"
    }))
}

#[tauri::command]
fn asteria_execute_command(command: String, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Ok(serde_json::json!({
            "status": "empty",
            "message": ""
        }));
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let cmd = parts[0].to_lowercase();

    match cmd.as_str() {
        "nav" | "navigate" | "open" | "g" | "goto" => {
            let url = if parts.len() > 1 { parts[1..].join(" ") } else { "https://example.com".to_string() };
            let mut mgr = state.tab_manager.lock().map_err(|e| e.to_string())?;
            mgr.navigate(&url).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "status": "success",
                "message": format!("[NAVIGATE] Active tab loaded target URL: {}", url)
            }))
        }
        "theme" | "mode" => {
            let mut theme = state.theme_mode.lock().map_err(|e| e.to_string())?;
            *theme = if *theme == "dark" { "light".to_string() } else { "dark".to_string() };
            Ok(serde_json::json!({
                "status": "success",
                "message": format!("[THEME] Active UI mode switched to Laboratory {}", theme.to_uppercase())
            }))
        }
        "telemetry" | "status" | "metrics" => {
            Ok(serde_json::json!({
                "status": "success",
                "message": "[TELEMETRY] FPS: 144 | Latency: 12ms | Kernel: AST-902 | Pipeline: 7-Stage Ready"
            }))
        }
        "help" => {
            Ok(serde_json::json!({
                "status": "info",
                "message": "[HELP] Available commands: navigate <url> | theme | telemetry | help"
            }))
        }
        _ => {
            Ok(serde_json::json!({
                "status": "executed",
                "message": format!("[DIAGNOSTIC] Executed query node: '{}'", trimmed)
            }))
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            tab_manager: Mutex::new(TabManager::new()),
            theme_mode: Mutex::new("dark".to_string()),
        })
        .invoke_handler(tauri::generate_handler![
            asteria_window_control,
            asteria_navigate,
            asteria_toggle_theme,
            asteria_get_telemetry,
            asteria_execute_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
