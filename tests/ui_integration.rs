use asteria::paint::DisplayList;
use asteria::scene::build_browser_ui_scene_graph;
use asteria::ui_theme::{LabTheme, ThemeMode};

#[test]
fn test_lab_theme_palettes() {
    let dark = LabTheme::dark();
    assert_eq!(dark.mode, ThemeMode::Dark);

    let light = LabTheme::light();
    assert_eq!(light.mode, ThemeMode::Light);

    assert_ne!(dark.surface, light.surface);
    assert_ne!(dark.background, light.background);
}

#[test]
fn test_browser_ui_scene_graph_generation() {
    let empty_dl = DisplayList::new();
    let theme = LabTheme::dark();
    let tabs = vec!["Console", "Performance", "Security"];
    let active_idx = 0;

    let scene = build_browser_ui_scene_graph(
        &empty_dl,
        256.0,
        1280.0,
        720.0,
        &theme,
        "https://asteria.engine/diagnostics",
        &tabs,
        active_idx,
        144,
        12,
    );

    assert!(!scene.is_empty(), "UI scene graph should contain chrome nodes");

    // Verify top bar and brand header text exists
    let has_brand = scene.texts.iter().flatten().any(|t| t.text.contains("ASTERIA // DIAGNOSTICS"));
    assert!(has_brand, "Scene graph should contain brand header text");

    // Verify window controls hit-test URLs exist
    let has_close = scene.nodes.iter().any(|n| n.link_url.as_deref() == Some("asteria://window/close"));
    let has_min = scene.nodes.iter().any(|n| n.link_url.as_deref() == Some("asteria://window/minimize"));
    let has_max = scene.nodes.iter().any(|n| n.link_url.as_deref() == Some("asteria://window/maximize"));

    assert!(has_close, "Scene graph should contain window close button");
    assert!(has_min, "Scene graph should contain window minimize button");
    assert!(has_max, "Scene graph should contain window maximize button");

    // Verify tab switch URLs exist
    let has_tab0 = scene.nodes.iter().any(|n| n.link_url.as_deref() == Some("asteria://tab/switch/0"));
    assert!(has_tab0, "Scene graph should contain tab 0 hit-test link");

    // Verify telemetry status footer text exists
    let has_telemetry = scene.texts.iter().flatten().any(|t| t.text.contains("FPS: 144") && t.text.contains("AST-902"));
    assert!(has_telemetry, "Scene graph should contain telemetry ticker text");
}

#[test]
fn test_browser_ui_hit_testing() {
    let empty_dl = DisplayList::new();
    let theme = LabTheme::dark();
    let tabs = vec!["Main"];

    let scene = build_browser_ui_scene_graph(
        &empty_dl,
        256.0,
        1280.0,
        720.0,
        &theme,
        "https://asteria.engine",
        &tabs,
        0,
        60,
        16,
    );

    // Hit test theme toggle button at top right (x = 1280 - 170 + 10 = 1120, y = 15)
    let hit_id = scene.hit_test(1120.0, 15.0);
    assert!(hit_id.is_some(), "Hit test should hit theme toggle node");
    if let Some(id) = hit_id {
        assert_eq!(scene.node_url(id), Some("asteria://theme/toggle"));
    }
}
