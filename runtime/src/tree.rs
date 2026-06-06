/// A node in the UI tree built while executing a Vel page.
#[derive(Debug, Clone)]
pub struct UiNode {
    pub tag: i32,
    pub tag_name: &'static str,
    pub props: Vec<UiProp>,
    pub text: Option<String>,
    pub children: Vec<UiNode>,
    /// WASM export name to call when this element is clicked (or Enter pressed on input).
    pub on_click: Option<String>,
    /// State variable binding key for input elements (format: "Page/varname").
    pub on_change: Option<String>,
    /// Props applied when the element is hovered.
    pub hover_props: Vec<UiProp>,
    /// Props applied when the element has keyboard focus.
    pub focus_props: Vec<UiProp>,
    /// Props applied while the element is being pressed (mousedown).
    pub active_props: Vec<UiProp>,
}

#[derive(Debug, Clone)]
pub struct UiProp {
    pub key: i32,
    pub key_name: &'static str,
    pub value: PropValue,
}

#[derive(Debug, Clone)]
pub enum PropValue {
    Number(f64),
    Bool(bool),
}

pub fn tag_name(tag: i32) -> &'static str {
    match tag {
        1 => "text",
        2 => "column",
        3 => "row",
        4 => "center",
        5 => "stack",
        6 => "grid",
        7 => "scroll",
        8 => "fixed",
        9 => "rect",
        10 => "image",
        11 => "icon",
        12 => "spacer",
        13 => "divider",
        14 => "spinner",
        15 => "Button",
        16 => "input",
        17 => "toast",
        _ => "element",
    }
}

pub fn prop_name(key: i32) -> &'static str {
    match key {
        1 => "padding",
        2 => "paddingTop",
        3 => "paddingBottom",
        4 => "paddingLeft",
        5 => "paddingRight",
        6 => "gap",
        7 => "size",
        8 => "weight",
        12 => "radius",
        13 => "border",
        14 => "width",
        15 => "height",
        16 => "bold",
        17 => "italic",
        18 => "color",
        19 => "background",
        20 => "lineHeight",
        21 => "align",
        22 => "opacity",
        23 => "shadow",
        24 => "overflow",
        25 => "letterSpacing",
        26 => "lines",
        27 => "borderColor",
        28 => "borderBottom",
        29 => "borderTop",
        30 => "borderLeft",
        31 => "borderRight",
        32 => "underline",
        33 => "strikethrough",
        34 => "animate",
        35 => "visible",
        36 => "cursor",
        37 => "columns",
        38 => "minWidth",
        _ => "prop",
    }
}
