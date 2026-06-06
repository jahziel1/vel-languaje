use super::Checker;
use super::scope::Scope;
use super::ty::Ty;

impl Checker {
    pub(super) fn builtin_scope(&self) -> Scope {
        let mut scope = Scope::new();

        for name in &[
            // Layout
            "text",
            "column",
            "row",
            "center",
            "stack",
            "grid",
            "scroll",
            "fixed",
            "rect",
            // Media
            "image",
            "icon",
            "spacer",
            "divider",
            // Stdlib components
            "Button",
            "spinner",
            "modal",
            "tooltip",
            "tabs",
            "tab",
            "accordion",
            "section",
            "list",
            "virtualList",
            // Inputs
            "input",
            "textarea",
            "checkbox",
            "toggle",
            "select",
            "datepicker",
            "filePicker",
            // Other UI
            "avatar",
            "badge",
            "outlet",
            "toast",
            // Navigation
            "go",
            "back",
            // Debug
            "print",
            // API and utils
            "api",
            "theme",
            "Math",
            "Date",
            // Slots
            "children",
            // Named colors
            "white",
            "black",
            "red",
            "green",
            "blue",
            "gray",
            "gray50",
            "gray100",
            "gray200",
            "gray300",
            "gray400",
            "gray500",
            "gray700",
            "gray900",
            "lightGray",
            "transparent",
            // Enum prop values — align
            "left",
            "right",
            "center",
            "top",
            "bottom",
            "spread",
            // Enum prop values — shadow
            "small",
            "medium",
            "large",
            // Enum prop values — overflow
            "clip",
            "ellipsis",
            // Enum prop values — cursor
            "pointer",
            "default",
            "grab",
            // Enum prop values — width/height
            "full",
            "half",
            "grow",
            "auto",
            // Enum prop values — scroll direction
            "horizontal",
            "vertical",
        ] {
            scope.define(*name, Ty::Unknown);
        }

        for variants in self.enum_defs.values() {
            for variant in variants {
                scope.define(variant.name.clone(), Ty::Unknown);
            }
        }

        for name in &self.store_names {
            scope.define(name.clone(), Ty::Unknown);
        }

        scope
    }
}
