// ── UI element tags ───────────────────────────────────────────────────────────

pub(crate) fn element_tag(name: &str) -> i32 {
    match name {
        "text" => 1,
        "column" => 2,
        "row" => 3,
        "center" => 4,
        "stack" => 5,
        "grid" => 6,
        "scroll" => 7,
        "fixed" => 8,
        "rect" => 9,
        "image" => 10,
        "icon" => 11,
        "spacer" => 12,
        "divider" => 13,
        "spinner" => 14,
        "Button" => 15,
        "input" => 16,
        "toast" => 17,
        _ => 99,
    }
}

pub(crate) fn is_ui_element(name: &str) -> bool {
    element_tag(name) != 99
}

// ── Prop key registry ─────────────────────────────────────────────────────────

pub(crate) fn prop_key(name: &str) -> i32 {
    match name {
        "padding" => 1,
        "paddingTop" => 2,
        "paddingBottom" => 3,
        "paddingLeft" => 4,
        "paddingRight" => 5,
        "gap" => 6,
        "size" | "fontSize" => 7,
        "weight" | "fontWeight" => 8,
        "radius" => 12,
        "border" => 13,
        "width" => 14,
        "height" => 15,
        "bold" => 16,
        "italic" => 17,
        // Visual props
        "color" => 18,
        "background" => 19,
        "lineHeight" => 20,
        "align" => 21,
        "opacity" => 22,
        "shadow" => 23,
        "overflow" => 24,
        "letterSpacing" => 25,
        "lines" => 26,
        "borderColor" => 27,
        "borderBottom" => 28,
        "borderTop" => 29,
        "borderLeft" => 30,
        "borderRight" => 31,
        "underline" => 32,
        "strikethrough" => 33,
        "animate" => 34,
        "visible" => 35,
        "cursor" => 36,
        "columns" => 37,
        "minWidth" => 38,
        _ => 99,
    }
}

pub(crate) fn is_bool_prop(name: &str) -> bool {
    matches!(
        name,
        "bold" | "italic" | "underline" | "strikethrough" | "animate" | "visible"
    )
}

pub(crate) fn is_color_prop(name: &str) -> bool {
    matches!(name, "color" | "background" | "borderColor")
}

// ── Color helpers ─────────────────────────────────────────────────────────────

pub(crate) fn parse_hex_rgb(hex: &str) -> u32 {
    let h = hex.trim_start_matches('#');
    let h6 = if h.len() >= 6 { &h[..6] } else { h };
    u32::from_str_radix(h6, 16).unwrap_or(0)
}

pub(crate) fn color_name_rgb(name: &str) -> Option<u32> {
    match name {
        "white" => Some(0xFF_FF_FF),
        "black" => Some(0x00_00_00),
        "red" => Some(0xEF_44_44),
        "green" => Some(0x10_B9_81),
        "blue" => Some(0x3B_82_F6),
        "gray" => Some(0x6B_72_80),
        "gray50" => Some(0xF9_FA_FB),
        "gray100" => Some(0xF3_F4_F6),
        "gray200" => Some(0xE5_E7_EB),
        "gray300" => Some(0xD1_D5_DB),
        "gray400" => Some(0x9C_A3_AF),
        "gray500" => Some(0x6B_72_80),
        "gray700" => Some(0x37_41_51),
        "gray900" => Some(0x11_18_27),
        "lightGray" => Some(0xD1_D5_DB),
        "transparent" => Some(0x00_00_00),
        _ => None,
    }
}

// ── Enum-style prop values ────────────────────────────────────────────────────

pub(crate) fn enum_prop_value(prop: &str, value: &str) -> Option<f64> {
    match (prop, value) {
        // align
        ("align", "left") => Some(0.0),
        ("align", "center") => Some(1.0),
        ("align", "right") => Some(2.0),
        ("align", "top") => Some(0.0),
        ("align", "bottom") => Some(2.0),
        ("align", "spread") => Some(3.0),
        // shadow
        ("shadow", "none") => Some(0.0),
        ("shadow", "small") => Some(1.0),
        ("shadow", "medium") => Some(2.0),
        ("shadow", "large") => Some(3.0),
        // overflow
        ("overflow", "visible") => Some(0.0),
        ("overflow", "clip") => Some(1.0),
        ("overflow", "ellipsis") => Some(2.0),
        // cursor
        ("cursor", "default") => Some(0.0),
        ("cursor", "pointer") => Some(1.0),
        ("cursor", "text") => Some(2.0),
        ("cursor", "grab") => Some(3.0),
        // columns keywords
        ("columns", "auto") => Some(-1.0),
        // width/height keywords
        ("width" | "height", "full") => Some(9999.0),
        ("width" | "height", "half") => Some(-50.0),
        ("width" | "height", "grow") => Some(-1.0),
        ("width" | "height", "auto") => Some(-2.0),
        _ => None,
    }
}
