/// Return a help hint for common mistakes when an identifier is not in scope.
pub(super) fn suggest_for_undefined(name: &str) -> Option<String> {
    if matches!(
        name,
        "bold" | "italic" | "underline" | "strikethrough" | "animate"
    ) {
        return Some(format!(
            "`{}` is a boolean property — use `{}: true` to enable it",
            name, name
        ));
    }
    if name == "visible" {
        return Some(
            "use `visible: true` to show or `visible: false` to hide an element".to_string(),
        );
    }
    if matches!(
        name,
        "white"
            | "black"
            | "red"
            | "green"
            | "blue"
            | "gray"
            | "gray50"
            | "gray100"
            | "gray200"
            | "gray300"
            | "gray400"
            | "gray500"
            | "gray700"
            | "gray900"
            | "lightGray"
            | "transparent"
    ) {
        return Some(format!(
            "`{}` is a color name — use `color: \"{}\"` or `background: \"{}\"`",
            name, name, name
        ));
    }
    if matches!(
        name,
        "left" | "center" | "right" | "top" | "bottom" | "spread"
    ) {
        return Some(format!(
            "`{}` is a layout keyword — use `align: {}` instead",
            name, name
        ));
    }
    if matches!(name, "pointer" | "grab") {
        return Some(format!(
            "`{}` is a cursor value — use `cursor: {}` instead",
            name, name
        ));
    }
    if matches!(name, "clip" | "ellipsis") {
        return Some(format!(
            "`{}` is an overflow value — use `overflow: {}` instead",
            name, name
        ));
    }
    if matches!(name, "small" | "medium" | "large") {
        return Some(format!(
            "`{}` is a size value — use `shadow: {}` for shadows",
            name, name
        ));
    }
    if matches!(name, "full" | "half" | "grow" | "auto") {
        return Some(format!(
            "`{}` is a size keyword — use `width: {}` or `height: {}` instead",
            name, name, name
        ));
    }
    None
}
