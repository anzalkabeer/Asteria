// ─── Milestone 6 Infrastructure & Scene Graph Integration Tests ──
//
// Tests for: SceneGraph, SegmentBuilder, LruCache, Pool, FrameArena,
// FrameBudget, TaskScheduler, and lazy image decoding.
// All tests in separate file per project convention.

use asteria::arena::FrameArena;
use asteria::cache::LruCache;
use asteria::frame::FrameBudget;
use asteria::layout::Rect;
use asteria::paint::{DisplayCommand, DisplayList};
use asteria::pool::Pool;
use asteria::scene::{
    NodeState, SceneGraph, SceneNode, SceneNodeId, SceneNodeKind, build_scene_graph,
};
use asteria::scheduler::{TaskPriority, TaskScheduler};
use asteria::segment::SegmentBuilder;
use asteria::values::Color;

// ─── Pool Tests ──────────────────────────────────────────────────

#[test]
fn test_pool_acquire_and_release() {
    let mut pool: Pool<u32> = Pool::new(5);
    assert_eq!(pool.available(), 5);

    let item = pool.acquire();
    assert_eq!(item, 0); // Default for u32
    assert_eq!(pool.available(), 4);

    pool.release(42);
    assert_eq!(pool.available(), 5);

    let reused = pool.acquire();
    assert_eq!(reused, 42); // Got the released item back
}

// ─── Arena Tests ─────────────────────────────────────────────────

#[test]
fn test_arena_alloc_and_reset() {
    let mut arena = FrameArena::new(1024);
    assert_eq!(arena.used(), 0);
    assert_eq!(arena.remaining(), 1024);

    let slice = arena.alloc(64);
    assert!(slice.is_some());
    assert_eq!(arena.used(), 64);

    // Reset clears everything in O(1)
    arena.reset();
    assert_eq!(arena.used(), 0);
    assert_eq!(arena.remaining(), 1024);
}

#[test]
fn test_arena_overflow_returns_none() {
    let mut arena = FrameArena::new(32);
    let big = arena.alloc(64);
    assert!(big.is_none());
}

// ─── LRU Cache Tests ────────────────────────────────────────────

#[test]
fn test_lru_cache_evicts_oldest() {
    let mut cache: LruCache<String, i32> = LruCache::new(3);

    cache.insert("a".into(), 1);
    cache.insert("b".into(), 2);
    cache.insert("c".into(), 3);
    assert_eq!(cache.len(), 3);

    // Inserting 4th item should evict "a" (oldest)
    cache.insert("d".into(), 4);
    assert_eq!(cache.len(), 3);
    assert!(cache.get(&"a".into()).is_none());
    assert!(cache.get(&"d".into()).is_some());
}

#[test]
fn test_lru_cache_access_refreshes_timestamp() {
    let mut cache: LruCache<String, i32> = LruCache::new(3);

    cache.insert("a".into(), 1);
    cache.insert("b".into(), 2);
    cache.insert("c".into(), 3);

    // Access "a" to refresh its timestamp
    let _ = cache.get(&"a".into());

    // Insert "d" — should evict "b" (now oldest), not "a"
    cache.insert("d".into(), 4);
    assert!(cache.get(&"a".into()).is_some()); // Still here
    assert!(cache.get(&"b".into()).is_none()); // Evicted
}

// ─── Frame Budget Tests ─────────────────────────────────────────

#[test]
fn test_frame_budget_60hz() {
    let mut budget = FrameBudget::new_60hz();
    assert!(!budget.is_over_budget());

    budget.input_ms = 2.0;
    budget.layout_ms = 5.0;
    budget.paint_ms = 3.0;
    budget.gpu_upload_ms = 3.0;
    budget.present_ms = 1.0;

    // 2 + 5 + 3 + 3 + 1 = 14ms < 16.67ms
    assert!(!budget.is_over_budget());
    assert!(budget.remaining() > 0.0);

    budget.paint_ms = 10.0; // Now 2+5+10+3+1 = 21ms > 16.67ms
    assert!(budget.is_over_budget());
}

// ─── Scheduler Tests ─────────────────────────────────────────────

#[test]
fn test_scheduler_priority_ordering() {
    let mut scheduler = TaskScheduler::new(4);

    scheduler.submit("low_task".into(), TaskPriority::Low);
    scheduler.submit("critical_task".into(), TaskPriority::Critical);
    scheduler.submit("normal_task".into(), TaskPriority::Normal);

    // Critical should come out first
    let first = scheduler.poll().unwrap();
    assert_eq!(first.name, "critical_task");

    let second = scheduler.poll().unwrap();
    assert_eq!(second.name, "normal_task");
}

#[test]
fn test_scheduler_adapts_to_workload() {
    let mut scheduler = TaskScheduler::new(8);

    scheduler.adapt_to_workload(10); // Simple page
    assert_eq!(scheduler.active_workers(), 1);

    scheduler.adapt_to_workload(1000); // Heavy page
    assert_eq!(scheduler.active_workers(), 4);

    scheduler.adapt_to_workload(5000); // Complex page
    assert_eq!(scheduler.active_workers(), 8);
}

// ─── Scene Graph Tests ───────────────────────────────────────────

#[test]
fn test_scene_graph_flat_storage() {
    let mut scene = SceneGraph::new();
    assert!(scene.is_empty());

    let id = scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: true,
            state: NodeState::Normal,
            link_url: None,
        },
        [1.0, 0.0, 0.0, 1.0],
        None,
    );

    assert_eq!(scene.len(), 1);
    assert_eq!(id, SceneNodeId(0));
}

#[test]
fn test_scene_graph_dirty_propagation() {
    let mut scene = SceneGraph::new();

    // Parent node
    let parent_id = scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
            kind: SceneNodeKind::Container,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [0.0; 4],
        None,
    );

    // Child node
    let child_id = scene.push(
        SceneNode {
            rect: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: Some(parent_id),
            z_order: 1,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [1.0, 0.0, 0.0, 1.0],
        None,
    );

    // Invalidate child — should propagate to parent
    scene.invalidate(child_id);
    assert!(scene.nodes[child_id.index()].dirty);
    assert!(scene.nodes[parent_id.index()].dirty);
    assert_eq!(scene.dirty_count(), 2);

    // Clear dirty flags
    scene.clear_dirty();
    assert_eq!(scene.dirty_count(), 0);
}

#[test]
fn test_build_scene_graph_from_display_list() {
    let mut list = DisplayList::default();
    list.commands.push(DisplayCommand::SolidColor {
        color: Color::new(255, 0, 0, 255), // Red
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        },
        link_url: None,
    });
    list.commands.push(DisplayCommand::Text {
        text: "Hello".to_string(),
        x: 100.0,
        y: 100.0,
        target_width: 800.0,
        font_size: 16.0,
        color: Color::new(255, 255, 255, 255),
        link_url: None,
    });

    let scene = build_scene_graph(&list, 256.0);
    assert_eq!(scene.len(), 2);

    // First node is SolidRect
    assert_eq!(scene.nodes[0].kind, SceneNodeKind::SolidRect);
    // Second node is Text
    assert!(matches!(scene.nodes[1].kind, SceneNodeKind::Text { .. }));
    // Both in segment 0 (y < 256)
    assert_eq!(scene.nodes[0].segment_id, 0);
    assert_eq!(scene.nodes[1].segment_id, 0);
}

// ─── Segment Builder Tests ───────────────────────────────────────

#[test]
fn test_segment_builder_divides_viewport() {
    let mut builder = SegmentBuilder::new(256.0);
    builder
        .build_segments(800.0, 1024.0)
        .expect("Invalid viewport dimensions");

    assert_eq!(builder.len(), 4); // 1024 / 256 = 4 segments
    assert_eq!(builder.segments[0].rect.y, 0.0);
    assert_eq!(builder.segments[1].rect.y, 256.0);
    assert_eq!(builder.segments[2].rect.y, 512.0);
    assert_eq!(builder.segments[3].rect.y, 768.0);

    // All dirty on first build
    assert_eq!(builder.dirty_segments().len(), 4);
}

#[test]
fn test_segment_builder_dirty_rect_intersection() {
    let mut builder = SegmentBuilder::new(256.0);
    builder
        .build_segments(800.0, 1024.0)
        .expect("Invalid viewport dimensions");

    // Mark all clean
    for i in 0..4 {
        builder.mark_clean(i);
    }
    assert_eq!(builder.dirty_segments().len(), 0);

    // Dirty a rect that overlaps segments 1 and 2
    builder.invalidate_rect(&Rect {
        x: 0.0,
        y: 300.0,
        width: 800.0,
        height: 300.0,
    });

    let dirty = builder.dirty_segments();
    assert!(dirty.contains(&1)); // y=256..512 overlaps 300..600
    assert!(dirty.contains(&2)); // y=512..768 overlaps 300..600
    assert!(!dirty.contains(&0));
    assert!(!dirty.contains(&3));
}

// ─── Lazy Image Decoding Tests ──────────────────────────────────

#[test]
fn test_lazy_decode_skips_offscreen_images() {
    let mut cache = asteria::image::ImageCache::new();
    let fake_png = [
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0,
        0, 50,
    ];

    let viewport = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };

    // Image far below viewport — should NOT decode
    let offscreen_rect = Rect {
        x: 0.0,
        y: 2000.0,
        width: 100.0,
        height: 50.0,
    };
    let result =
        cache.get_or_decode_if_visible("offscreen.png", &fake_png, &offscreen_rect, &viewport);
    assert!(result.unwrap().is_none());
    assert_eq!(cache.len(), 0); // Nothing cached

    // Image inside viewport — SHOULD decode
    let onscreen_rect = Rect {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 50.0,
    };
    let result =
        cache.get_or_decode_if_visible("onscreen.png", &fake_png, &onscreen_rect, &viewport);
    assert!(result.unwrap().is_some());
    assert_eq!(cache.len(), 1);
}

// ─── Phase 9C: Hit Testing Tests ─────────────────────────────────

#[test]
fn test_hit_test_finds_topmost_node() {
    let mut scene = SceneGraph::new();

    // Background rect covering entire area z_order=0
    scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [0.9, 0.9, 0.9, 1.0],
        None,
    );

    // Foreground button rect z_order=1
    let button_id = scene.push(
        SceneNode {
            rect: Rect {
                x: 100.0,
                y: 100.0,
                width: 200.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 1,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [0.2, 0.5, 1.0, 1.0],
        None,
    );

    // Click inside the button area → should return button (higher z_order)
    let hit = scene.hit_test(150.0, 120.0);
    assert_eq!(
        hit,
        Some(button_id),
        "Should hit topmost (highest z_order) node"
    );
}

#[test]
fn test_hit_test_miss_returns_none() {
    let mut scene = SceneGraph::new();

    scene.push(
        SceneNode {
            rect: Rect {
                x: 100.0,
                y: 100.0,
                width: 50.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [1.0, 0.0, 0.0, 1.0],
        None,
    );

    // Point outside all nodes
    let hit = scene.hit_test(500.0, 500.0);
    assert!(
        hit.is_none(),
        "Should return None when nothing is under cursor"
    );
}

#[test]
fn test_hit_test_edge_boundary() {
    let mut scene = SceneGraph::new();

    let id = scene.push(
        SceneNode {
            rect: Rect {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 100.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [0.0, 1.0, 0.0, 1.0],
        None,
    );

    // Exact top-left corner — inside
    assert_eq!(scene.hit_test(50.0, 50.0), Some(id));
    // Exact bottom-right corner — inside
    assert_eq!(scene.hit_test(150.0, 150.0), Some(id));
    // Just outside right edge — miss
    assert_eq!(scene.hit_test(151.0, 100.0), None);
}

#[test]
fn test_invalidate_after_hit() {
    let mut scene = SceneGraph::new();

    let id = scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [1.0, 1.0, 0.0, 1.0],
        None,
    );

    // Node starts clean
    assert!(!scene.nodes[id.index()].dirty);

    // Simulate click → invalidate
    let hit = scene.hit_test(50.0, 50.0);
    if let Some(node_id) = hit {
        scene.invalidate(node_id);
    }

    // Node should now be dirty
    assert!(scene.nodes[id.index()].dirty);
}

// ─── Milestone 10 Component 1 Tests ───────────────────────────────

#[test]
fn test_node_state_invalidation() {
    let mut scene = SceneGraph::new();

    let id = scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 2,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [1.0, 0.0, 0.0, 1.0],
        None,
    );

    assert_eq!(scene.nodes[id.index()].state, NodeState::Normal);
    assert!(!scene.nodes[id.index()].dirty);

    // Change state to Hovered → should mark dirty
    let changed = scene.set_node_state(id, NodeState::Hovered);
    assert!(changed);
    assert_eq!(scene.nodes[id.index()].state, NodeState::Hovered);
    assert!(scene.nodes[id.index()].dirty);

    // Setting same state again → returns false, no redundant invalidation
    scene.clear_dirty();
    let changed_again = scene.set_node_state(id, NodeState::Hovered);
    assert!(!changed_again);
    assert!(!scene.nodes[id.index()].dirty);
}

#[test]
fn test_dirty_segments_query() {
    let mut scene = SceneGraph::new();

    // Node in segment 0 (clean)
    scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [0.0; 4],
        None,
    );

    // Node in segment 2 (dirty)
    let n2 = scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 500.0,
                width: 100.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 1,
            segment_id: 2,
            dirty: true,
            state: NodeState::Normal,
            link_url: None,
        },
        [0.0; 4],
        None,
    );

    let dirty_segs = scene.dirty_segments();
    assert_eq!(dirty_segs, vec![2], "Should isolate dirty segment 2");
    assert_eq!(n2.index(), 1);
}

#[test]
fn test_node_link_url_retrieval() {
    let mut scene = SceneGraph::new();

    let id = scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            kind: SceneNodeKind::Text { font_size: 16.0 },
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: Some("https://asteria.dev".to_string()),
        },
        [0.0; 4],
        None,
    );

    assert_eq!(scene.node_url(id), Some("https://asteria.dev"));
}

#[test]
fn test_clean_scene_empty_dirty_segments() {
    let mut scene = SceneGraph::new();

    scene.push(
        SceneNode {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: false,
            state: NodeState::Normal,
            link_url: None,
        },
        [0.0; 4],
        None,
    );

    assert!(
        scene.dirty_segments().is_empty(),
        "Clean scene should return 0 dirty segments"
    );
}
