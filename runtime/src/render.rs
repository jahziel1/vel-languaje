use crate::tree::{PropValue, UiNode};

/// Render a UI tree as an indented debug string. Useful for tests and `vel run`.
pub fn render_debug(nodes: &[UiNode]) -> String {
    let mut out = String::new();
    render_nodes(nodes, 0, &mut out);
    out
}

fn render_nodes(nodes: &[UiNode], depth: usize, out: &mut String) {
    for node in nodes {
        let indent = "  ".repeat(depth);
        let props: String = node
            .props
            .iter()
            .map(|p| match p.value {
                PropValue::Number(n) => format!(" {}={}", p.key_name, n),
                PropValue::Bool(b) => format!(" {}={}", p.key_name, b),
            })
            .collect();
        let text = node
            .text
            .as_ref()
            .map(|t| {
                if t.is_empty() {
                    String::new()
                } else {
                    format!(" \"{}\"", t)
                }
            })
            .unwrap_or_default();

        if node.children.is_empty() {
            out.push_str(&format!("{}<{}{}{}/>", indent, node.tag_name, text, props));
        } else {
            out.push_str(&format!("{}<{}{}{}>", indent, node.tag_name, text, props));
            out.push('\n');
            render_nodes(&node.children, depth + 1, out);
            out.push_str(&format!("{}</{}>", indent, node.tag_name));
        }
        out.push('\n');
    }
}
