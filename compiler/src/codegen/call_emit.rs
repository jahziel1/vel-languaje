use wasm_encoder::Instruction;

use crate::parser::ast::{Arg, Expr, ObjectEntry, Stmt};

use super::emit::Ctx;
use super::text_arg_emit::extract_static_string;
use super::{
    CodeGen, RT_BEGIN, RT_END, RT_NAV_PARAM, RT_NAVIGATE, RT_PRINT_BUILT, RT_PRINT_STR,
    RT_PROP_BOOL, RT_PROP_F64, RT_SET_ON_CHANGE, RT_SET_ON_CLICK, RT_STATE_POP, RT_STATE_PUSH,
    RT_STR_BEGIN, RT_TEXT_CONTENT, element_tag, enum_prop_value, is_bool_prop, is_color_prop,
    is_ui_element, prop_key,
};

impl CodeGen {
    pub(super) fn emit_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Arg],
        block: &Option<Vec<Stmt>>,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let fn_name = match callee {
            Expr::Ident(name) => name.clone(),
            Expr::Field(obj, method) => {
                // data.sum(item -> item.field) — host-side aggregation
                if method == "sum"
                    && let Expr::Ident(var_name) = obj.as_ref()
                    && let Some(&api_local) = ctx.api_bindings.get(var_name.as_str())
                {
                    self.emit_list_sum(api_local, args, ctx, out);
                    return;
                }
                // store.fn() — call exported store function
                if let Expr::Ident(store_name) = obj.as_ref()
                    && self.store_names.contains(store_name.as_str())
                {
                    let store_fn_key = format!("store_{}_{}", store_name, method);
                    let maybe_fn = self.func_map.get(&store_fn_key).copied();
                    if let Some(fn_idx) = maybe_fn {
                        for arg in args {
                            self.emit_expr_f64(&arg.value, ctx, out);
                        }
                        out.push(Instruction::Call(fn_idx));
                    }
                    out.push(Instruction::I32Const(0));
                    return;
                }
                self.emit_expr_push(obj, ctx, out);
                out.push(Instruction::Drop);
                out.push(Instruction::I32Const(0));
                return;
            }
            _ => {
                out.push(Instruction::I32Const(0));
                return;
            }
        };

        if fn_name == "list" {
            self.emit_list_call(args, block, ctx, out);
            return;
        }

        if fn_name == "print" {
            if let Some(arg) = args.first() {
                if let Some(s) = extract_static_string(&arg.value) {
                    let (ptr, len) = self.intern(&s);
                    out.push(Instruction::I32Const(ptr as i32));
                    out.push(Instruction::I32Const(len as i32));
                    out.push(Instruction::Call(RT_PRINT_STR));
                } else {
                    // Interpolated string: build via str_begin/parts, then print_str_built
                    out.push(Instruction::Call(RT_STR_BEGIN));
                    self.emit_text_arg_parts(&arg.value, ctx, out);
                    out.push(Instruction::Call(RT_PRINT_BUILT));
                }
            }
            out.push(Instruction::I32Const(0));
            return;
        }

        if fn_name == "go" {
            let path = args.first().and_then(|a| match &a.value {
                Expr::Ident(name) if name == "back" => Some("back".to_owned()),
                other => extract_static_string(other),
            });
            if let Some(p) = path {
                // Emit navigation parameters before calling navigate.
                for (idx, arg) in args[1..].iter().enumerate() {
                    out.push(Instruction::I32Const(idx as i32));
                    self.emit_expr_f64(&arg.value, ctx, out);
                    out.push(Instruction::Call(RT_NAV_PARAM));
                }
                let (ptr, len) = self.intern(&p);
                out.push(Instruction::I32Const(ptr as i32));
                out.push(Instruction::I32Const(len as i32));
                out.push(Instruction::Call(RT_NAVIGATE));
            }
            out.push(Instruction::I32Const(0));
            return;
        }

        if is_ui_element(&fn_name) {
            out.push(Instruction::I32Const(element_tag(&fn_name)));
            out.push(Instruction::Call(RT_BEGIN));

            let is_input = fn_name == "input";

            for arg in args {
                if let Some(prop_name) = &arg.name {
                    if prop_name == "onClick" || (is_input && prop_name == "onEnter") {
                        if let Expr::Ident(handler) = &arg.value {
                            let export = format!("{}_{}", ctx.page, handler);
                            let (ptr, len) = self.intern(&export);
                            out.push(Instruction::I32Const(ptr as i32));
                            out.push(Instruction::I32Const(len as i32));
                            out.push(Instruction::Call(RT_SET_ON_CLICK));
                        }
                    } else if is_input && prop_name == "placeholder" {
                        let (ptr, len) = extract_static_string(&arg.value)
                            .map(|s| {
                                let (p, l) = self.intern(&s);
                                (p as i32, l as i32)
                            })
                            .unwrap_or((0, 0));
                        out.push(Instruction::I32Const(ptr));
                        out.push(Instruction::I32Const(len));
                        out.push(Instruction::Call(RT_TEXT_CONTENT));
                    } else if matches!(prop_name.as_str(), "hover" | "focus" | "active") {
                        let kind: i32 = match prop_name.as_str() {
                            "hover" => 1,
                            "focus" => 2,
                            _ => 3, // "active"
                        };
                        if let Expr::Object(entries) = &arg.value {
                            out.push(Instruction::I32Const(kind));
                            out.push(Instruction::Call(RT_STATE_PUSH));
                            self.emit_state_props(entries, ctx, out);
                            out.push(Instruction::Call(RT_STATE_POP));
                        }
                    } else if is_color_prop(prop_name) {
                        let rgb = self.resolve_color_rgb(&arg.value);
                        out.push(Instruction::I32Const(prop_key(prop_name)));
                        out.push(Instruction::F64Const((rgb as f64).into()));
                        out.push(Instruction::Call(RT_PROP_F64));
                    } else if is_bool_prop(prop_name) {
                        out.push(Instruction::I32Const(prop_key(prop_name)));
                        self.emit_expr_i32(&arg.value, ctx, out);
                        out.push(Instruction::Call(RT_PROP_BOOL));
                    } else {
                        let key = prop_key(prop_name);
                        let enum_val = if let Expr::Ident(val) = &arg.value {
                            enum_prop_value(prop_name, val)
                        } else {
                            None
                        };
                        out.push(Instruction::I32Const(key));
                        if let Some(n) = enum_val {
                            out.push(Instruction::F64Const(n.into()));
                        } else {
                            self.emit_expr_f64(&arg.value, ctx, out);
                        }
                        out.push(Instruction::Call(RT_PROP_F64));
                    }
                } else if is_input {
                    if let Expr::Ident(var_name) = &arg.value {
                        let binding = format!("{}/{}", ctx.page, var_name);
                        let (ptr, len) = self.intern(&binding);
                        out.push(Instruction::I32Const(ptr as i32));
                        out.push(Instruction::I32Const(len as i32));
                        out.push(Instruction::Call(RT_SET_ON_CHANGE));
                    }
                } else {
                    self.emit_text_arg(&arg.value, ctx, out);
                }
            }

            if let Some(child_stmts) = block {
                for s in child_stmts {
                    self.emit_stmt(s, ctx, out);
                }
            }

            out.push(Instruction::Call(RT_END));
            out.push(Instruction::I32Const(0));
        } else {
            for arg in args {
                self.emit_expr_f64(&arg.value, ctx, out);
            }
            let fn_key_prefixed = format!("{}_{}", ctx.page, fn_name);

            let component_key = format!("component_{}", fn_name);
            if let Some(&fn_idx) = self
                .func_map
                .get(&fn_key_prefixed)
                .or_else(|| self.func_map.get(fn_name.as_str()))
                .or_else(|| self.func_map.get(&component_key))
            {
                out.push(Instruction::Call(fn_idx));
            } else {
                for _ in args {
                    out.push(Instruction::Drop);
                }
            }
            out.push(Instruction::I32Const(0));
        }
    }

    /// Emit prop_f64/prop_bool calls for each entry in a hover/focus/active state block.
    pub(super) fn emit_state_props(
        &mut self,
        entries: &[ObjectEntry],
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        for entry in entries {
            if let ObjectEntry::Field(name, val) = entry {
                if is_color_prop(name) {
                    let rgb = self.resolve_color_rgb(val);
                    out.push(Instruction::I32Const(prop_key(name)));
                    out.push(Instruction::F64Const((rgb as f64).into()));
                    out.push(Instruction::Call(RT_PROP_F64));
                } else if is_bool_prop(name) {
                    out.push(Instruction::I32Const(prop_key(name)));
                    self.emit_expr_i32(val, ctx, out);
                    out.push(Instruction::Call(RT_PROP_BOOL));
                } else {
                    let key = prop_key(name);
                    let enum_val = if let Expr::Ident(v) = val {
                        enum_prop_value(name, v)
                    } else {
                        None
                    };
                    out.push(Instruction::I32Const(key));
                    if let Some(n) = enum_val {
                        out.push(Instruction::F64Const(n.into()));
                    } else {
                        self.emit_expr_f64(val, ctx, out);
                    }
                    out.push(Instruction::Call(RT_PROP_F64));
                }
            }
        }
    }
}
