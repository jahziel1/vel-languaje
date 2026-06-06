use crate::tree::{prop_f32, UiNode};

#[derive(Clone, Debug)]
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub children: Vec<LayoutBox>,
}

fn leaf_h(tag: u32) -> f32 {
    match tag {
        1 => 22.0,  // text
        13 => 1.0,  // divider
        14 => 40.0, // spinner
        15 => 38.0, // Button
        16 => 38.0, // input
        _ => 40.0,
    }
}

pub fn node_layout(node: &UiNode, x: f32, y: f32, avail_w: f32) -> LayoutBox {
    let p = &node.props;
    let pad = prop_f32(p, "padding").unwrap_or(0.0);
    let pt = prop_f32(p, "paddingTop").unwrap_or(pad);
    let pb = prop_f32(p, "paddingBottom").unwrap_or(pad);
    let pl = prop_f32(p, "paddingLeft").unwrap_or(pad);
    let pr = prop_f32(p, "paddingRight").unwrap_or(pad);
    let gap = prop_f32(p, "gap").unwrap_or(0.0);
    let w = prop_f32(p, "width").unwrap_or(avail_w);
    let inner_w = (w - pl - pr).max(0.0);

    if node.children.is_empty() {
        return LayoutBox {
            x,
            y,
            w,
            h: prop_f32(p, "height").unwrap_or(leaf_h(node.tag)),
            children: vec![],
        };
    }

    // row
    if node.tag == 3 {
        let n = node.children.len();
        let cw = if n > 0 {
            (inner_w - gap * (n.saturating_sub(1)) as f32) / n as f32
        } else {
            inner_w
        };
        let mut children = Vec::with_capacity(n);
        let mut cx = x + pl;
        let mut max_h = 0.0f32;
        for (i, child) in node.children.iter().enumerate() {
            let b = node_layout(child, cx, y + pt, cw);
            max_h = max_h.max(b.h);
            cx += b.w + if i + 1 < n { gap } else { 0.0 };
            children.push(b);
        }
        return LayoutBox {
            x,
            y,
            w,
            h: prop_f32(p, "height").unwrap_or(max_h + pt + pb),
            children,
        };
    }

    // column / center / other — vertical stack
    let mut children = Vec::with_capacity(node.children.len());
    let mut cy = y + pt;
    let n = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let b = node_layout(child, x + pl, cy, inner_w);
        cy += b.h + if i + 1 < n { gap } else { 0.0 };
        children.push(b);
    }
    LayoutBox {
        x,
        y,
        w,
        h: prop_f32(p, "height").unwrap_or(cy + pb - y),
        children,
    }
}

pub fn layout_root(nodes: &[UiNode], avail_w: f32) -> Vec<LayoutBox> {
    let mut boxes = Vec::with_capacity(nodes.len());
    let mut cy = 0.0f32;
    for node in nodes {
        let b = node_layout(node, 0.0, cy, avail_w);
        cy += b.h + 8.0;
        boxes.push(b);
    }
    boxes
}
