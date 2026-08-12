use asteria::renderer::window::chrome::ChromeShell;
use asteria::shell::TabManager;
use asteria::ui_theme::ThemeMode;
use asteria::viewport::EngineViewport;

#[test]
fn test_chrome_shell_scene_generation() {
    let chrome = ChromeShell::new();
    let tab_manager = TabManager::new();

    let scene = chrome.build_chrome_scene(800.0, 600.0, &tab_manager);
    assert!(!scene.nodes.is_empty(), "Chrome scene graph should generate non-empty UI nodes");

    // Verify z_order >= 1000 for Chrome nodes
    let all_high_z = scene.nodes.iter().all(|n| n.z_order >= 1000);
    assert!(all_high_z, "All Chrome nodes must have z_order >= 1000");
}

#[test]
fn test_chrome_theme_toggle() {
    let mut chrome = ChromeShell::new();
    assert_eq!(chrome.theme.mode, ThemeMode::Dark);

    chrome.toggle_theme();
    assert_eq!(chrome.theme.mode, ThemeMode::Light);

    chrome.toggle_theme();
    assert_eq!(chrome.theme.mode, ThemeMode::Dark);
}

#[test]
fn test_engine_viewport_scene_generation() {
    let viewport = EngineViewport::new();
    let tab_manager = TabManager::new();

    let scene = viewport.build_page_scene(800.0, 600.0, &tab_manager);
    assert!(!scene.nodes.is_empty(), "Viewport page scene graph should generate nodes");

    // Verify page nodes are offset by top header (y >= 84.0)
    let shifted = scene.nodes.iter().all(|n| n.rect.y >= 84.0);
    assert!(shifted, "Viewport page nodes must be offset below top navigation bar (y >= 84.0)");
}
