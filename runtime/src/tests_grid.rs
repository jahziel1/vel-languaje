use crate::layout::layout;
use crate::tree::{PropValue, UiNode, UiProp};

fn make_text() -> UiNode {
    UiNode {
        tag: 1,
        tag_name: "text",
        props: vec![],
        text: Some("x".into()),
        children: vec![],
        on_click: None,
        on_change: None,
        hover_props: vec![],
        focus_props: vec![],
        active_props: vec![],
    }
}

fn grid_node(
    cols_val: f64,
    gap_val: f64,
    extra_props: Vec<UiProp>,
    children: Vec<UiNode>,
) -> UiNode {
    let mut props = vec![
        UiProp {
            key: 37,
            key_name: "columns",
            value: PropValue::Number(cols_val),
        },
        UiProp {
            key: 6,
            key_name: "gap",
            value: PropValue::Number(gap_val),
        },
    ];
    props.extend(extra_props);
    UiNode {
        tag: 6,
        tag_name: "grid",
        props,
        text: None,
        children,
        on_click: None,
        on_change: None,
        hover_props: vec![],
        focus_props: vec![],
        active_props: vec![],
    }
}

#[test]
fn test_grid_fixed_3_cols_positions() {
    let grid = grid_node(
        3.0,
        0.0,
        vec![],
        vec![make_text(), make_text(), make_text(), make_text()],
    );
    // 300px wide, 3 cols, no gap → each cell 100px
    let b = layout(&grid, 0.0, 0.0, 300.0);
    assert_eq!(b.children.len(), 4);
    assert!((b.children[0].x - 0.0).abs() < 0.01);
    assert!((b.children[1].x - 100.0).abs() < 0.01);
    assert!((b.children[2].x - 200.0).abs() < 0.01);
    // row 1
    assert!((b.children[3].x - 0.0).abs() < 0.01);
    assert!(b.children[3].y > b.children[0].y);
}

#[test]
fn test_grid_gap_applied_between_cols() {
    // 3 cols, gap=10, avail=320 → cell_w = (320 - 20) / 3 ≈ 100
    let grid = grid_node(
        3.0,
        10.0,
        vec![],
        vec![make_text(), make_text(), make_text()],
    );
    let b = layout(&grid, 0.0, 0.0, 320.0);
    assert_eq!(b.children.len(), 3);
    assert!((b.children[0].x - 0.0).abs() < 0.01);
    let cell_w = (320.0_f64 - 20.0) / 3.0;
    assert!((b.children[1].x - (cell_w + 10.0)).abs() < 0.01);
    assert!((b.children[2].x - (cell_w * 2.0 + 20.0)).abs() < 0.01);
}

#[test]
fn test_grid_auto_cols_300px_minwidth_100() {
    // auto, minWidth=100, gap=0, avail=300 → 3 cols
    let grid = grid_node(
        -1.0,
        0.0,
        vec![UiProp {
            key: 38,
            key_name: "minWidth",
            value: PropValue::Number(100.0),
        }],
        vec![make_text(), make_text(), make_text()],
    );
    let b = layout(&grid, 0.0, 0.0, 300.0);
    assert_eq!(b.children.len(), 3);
    assert!((b.children[0].x - 0.0).abs() < 0.01);
    assert!((b.children[1].x - 100.0).abs() < 0.01);
    assert!((b.children[2].x - 200.0).abs() < 0.01);
}

#[test]
fn test_grid_auto_cols_small_container() {
    // auto, minWidth=200, gap=0, avail=350 → 1 col (350/200 = 1.75 → floor = 1)
    let grid = grid_node(
        -1.0,
        0.0,
        vec![UiProp {
            key: 38,
            key_name: "minWidth",
            value: PropValue::Number(200.0),
        }],
        vec![make_text(), make_text()],
    );
    let b = layout(&grid, 0.0, 0.0, 350.0);
    // 1 col means 2 rows
    assert_eq!(b.children.len(), 2);
    // both cells at x=0
    assert!((b.children[0].x - 0.0).abs() < 0.01);
    assert!((b.children[1].x - 0.0).abs() < 0.01);
    assert!(b.children[1].y > b.children[0].y);
}

#[test]
fn test_grid_empty_no_crash() {
    let grid = grid_node(3.0, 8.0, vec![], vec![]);
    let b = layout(&grid, 0.0, 0.0, 300.0);
    assert_eq!(b.children.len(), 0);
}
