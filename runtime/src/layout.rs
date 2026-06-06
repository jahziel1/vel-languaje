use crate::tree::{PropValue, UiNode, UiProp};

// ── Computed layout box for one UiNode ───────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub children: Vec<LayoutBox>,
}

// ── Prop helpers ──────────────────────────────────────────────────────────────

fn get_f64(props: &[UiProp], key: i32) -> Option<f64> {
    props.iter().find(|p| p.key == key).and_then(|p| {
        if let PropValue::Number(v) = p.value {
            Some(v)
        } else {
            None
        }
    })
}

fn padding(props: &[UiProp]) -> (f64, f64, f64, f64) {
    let all = get_f64(props, 1).unwrap_or(0.0);
    let top = get_f64(props, 2).unwrap_or(all);
    let bottom = get_f64(props, 3).unwrap_or(all);
    let left = get_f64(props, 4).unwrap_or(all);
    let right = get_f64(props, 5).unwrap_or(all);
    (top, bottom, left, right)
}

// ── Leaf intrinsic height ─────────────────────────────────────────────────────

fn leaf_height(tag: i32) -> f64 {
    match tag {
        1 => 22.0,  // text
        13 => 1.0,  // divider
        14 => 40.0, // spinner
        15 => 38.0, // Button
        _ => 32.0,
    }
}

// ── Main layout pass ──────────────────────────────────────────────────────────

pub fn layout(node: &UiNode, x: f64, y: f64, avail_w: f64) -> LayoutBox {
    let (pt, pb, pl, pr) = padding(&node.props);
    let gap = get_f64(&node.props, 6).unwrap_or(0.0);
    let fixed_w = get_f64(&node.props, 14);
    let fixed_h = get_f64(&node.props, 15);

    let w = fixed_w.unwrap_or(avail_w);
    let inner_w = (w - pl - pr).max(0.0);

    if node.children.is_empty() {
        let h = fixed_h.unwrap_or_else(|| leaf_height(node.tag));
        return LayoutBox {
            x,
            y,
            w,
            h,
            children: vec![],
        };
    }

    match node.tag {
        // column / center — stack children vertically
        2 | 4 => {
            let mut children = Vec::with_capacity(node.children.len());
            let mut cur_y = y + pt;
            for (i, child) in node.children.iter().enumerate() {
                let b = layout(child, x + pl, cur_y, inner_w);
                cur_y += b.h;
                if i + 1 < node.children.len() {
                    cur_y += gap;
                }
                children.push(b);
            }
            let h = fixed_h.unwrap_or(cur_y + pb - y);
            LayoutBox {
                x,
                y,
                w,
                h,
                children,
            }
        }
        // row — stack children horizontally
        3 => {
            let n = node.children.len() as f64;
            let child_w = if n > 0.0 {
                (inner_w - gap * (n - 1.0).max(0.0)) / n
            } else {
                inner_w
            };
            let mut children = Vec::with_capacity(node.children.len());
            let mut cur_x = x + pl;
            let mut max_h: f64 = 0.0;
            for (i, child) in node.children.iter().enumerate() {
                let b = layout(child, cur_x, y + pt, child_w);
                max_h = max_h.max(b.h);
                cur_x += b.w;
                if i + 1 < node.children.len() {
                    cur_x += gap;
                }
                children.push(b);
            }
            let h = fixed_h.unwrap_or(max_h + pt + pb);
            LayoutBox {
                x,
                y,
                w,
                h,
                children,
            }
        }
        // grid — multi-column layout
        6 => {
            let cols_prop = get_f64(&node.props, 37).unwrap_or(1.0);
            let min_width = get_f64(&node.props, 38).unwrap_or(100.0);
            let cols = if cols_prop < 0.0 {
                // auto: fit as many columns as possible given minWidth
                ((inner_w + gap) / (min_width + gap)).floor().max(1.0) as usize
            } else {
                cols_prop.max(1.0) as usize
            };
            let cell_w = if cols > 1 {
                (inner_w - gap * (cols - 1) as f64) / cols as f64
            } else {
                inner_w
            };

            let n = node.children.len();
            let rows = n.div_ceil(cols);
            let mut children = Vec::with_capacity(n);
            let mut cur_y = y + pt;

            for row in 0..rows {
                let row_start = row * cols;
                let row_end = (row_start + cols).min(n);
                let mut row_h = 0.0f64;
                let mut row_boxes = Vec::with_capacity(cols);
                for (ci, idx) in (row_start..row_end).enumerate() {
                    let cell_x = x + pl + ci as f64 * (cell_w + gap);
                    let b = layout(&node.children[idx], cell_x, cur_y, cell_w);
                    row_h = row_h.max(b.h);
                    row_boxes.push(b);
                }
                cur_y += row_h;
                if row + 1 < rows {
                    cur_y += gap;
                }
                children.extend(row_boxes);
            }

            let h = fixed_h.unwrap_or(cur_y + pb - y);
            LayoutBox {
                x,
                y,
                w,
                h,
                children,
            }
        }
        // scroll / stack — treat as column
        _ => {
            let mut children = Vec::with_capacity(node.children.len());
            let mut cur_y = y + pt;
            for (i, child) in node.children.iter().enumerate() {
                let b = layout(child, x + pl, cur_y, inner_w);
                cur_y += b.h;
                if i + 1 < node.children.len() {
                    cur_y += gap;
                }
                children.push(b);
            }
            let h = fixed_h.unwrap_or(cur_y + pb - y);
            LayoutBox {
                x,
                y,
                w,
                h,
                children,
            }
        }
    }
}

/// Walk nodes+boxes and return the `on_click` handler of the deepest hit element.
pub fn hit_test(nodes: &[UiNode], boxes: &[LayoutBox], mx: f64, my: f64) -> Option<String> {
    for (node, b) in nodes.iter().zip(boxes.iter()) {
        if mx >= b.x && mx < b.x + b.w && my >= b.y && my < b.y + b.h {
            if !node.children.is_empty()
                && let Some(s) = hit_test(&node.children, &b.children, mx, my)
            {
                return Some(s);
            }
            if let Some(fn_name) = &node.on_click {
                return Some(fn_name.clone());
            }
        }
    }
    None
}

/// Walk nodes+boxes and return the `on_change` binding of the deepest input element hit.
pub fn find_input_at(nodes: &[UiNode], boxes: &[LayoutBox], mx: f64, my: f64) -> Option<String> {
    for (node, b) in nodes.iter().zip(boxes.iter()) {
        if mx >= b.x && mx < b.x + b.w && my >= b.y && my < b.y + b.h {
            if !node.children.is_empty()
                && let Some(s) = find_input_at(&node.children, &b.children, mx, my)
            {
                return Some(s);
            }
            if node.tag == 16 {
                return node.on_change.clone();
            }
        }
    }
    None
}

/// Walk the node tree and find the `on_click` handler of an input bound to `var_name`.
pub fn find_enter_handler(nodes: &[UiNode], var_name: &str) -> Option<String> {
    for node in nodes {
        if node.tag == 16 && node.on_change.as_deref() == Some(var_name) {
            return node.on_click.clone();
        }
        if let Some(result) = find_enter_handler(&node.children, var_name) {
            return Some(result);
        }
    }
    None
}

/// Inject current input values into input nodes after WASM render.
pub fn patch_input_values(
    nodes: &mut [UiNode],
    input_values: &std::collections::HashMap<String, String>,
) {
    for node in nodes.iter_mut() {
        if node.tag == 16
            && let Some(var_name) = &node.on_change
            && let Some(value) = input_values.get(var_name)
        {
            node.text = Some(value.clone());
        }
        patch_input_values(&mut node.children, input_values);
    }
}

/// Compute layout for the root node list (top-level nodes stacked vertically).
pub fn layout_root(nodes: &[UiNode], x: f64, y: f64, avail_w: f64) -> Vec<LayoutBox> {
    let mut boxes = Vec::with_capacity(nodes.len());
    let mut cur_y = y;
    for node in nodes {
        let b = layout(node, x, cur_y, avail_w);
        cur_y += b.h + 8.0;
        boxes.push(b);
    }
    boxes
}
