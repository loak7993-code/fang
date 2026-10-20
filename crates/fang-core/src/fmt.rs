//! Canonical formatter. One style, no configuration.
//!
//! Hard property, tested: format is a fixpoint. Re-parsing formatted
//! output and formatting it again yields identical text.

use crate::ast::*;

pub fn format_module(m: &Module) -> String {
    let mut out = String::new();
    let mut prev_was_import = false;
    let mut first = true;
    for item in &m.items {
        match item {
            Item::Import { module, path, .. } => {
                if !first && !prev_was_import {
                    out.push('\n');
                }
                match path {
                    Some(p) => out.push_str(&format!("import {} from \"{}\"\n", module, escape_str(p))),
                    None => out.push_str(&format!("import {}\n", module)),
                }
                prev_was_import = true;
            }
            Item::Const { name, ann, value, exported, .. } => {
                if !first {
                    out.push('\n');
                }
                let kw = if *exported { "export let" } else { "let" };
                out.push_str(&format!("{} {}", kw, name));
                if let Some(a) = ann {
                    out.push_str(&format!(": {}", a));
                }
                out.push_str(" = ");
                write_expr(&mut out, value, 0);
                out.push('\n');
                prev_was_import = false;
            }
            Item::Fn(f) => {
                if !first {
                    out.push('\n');
                }
                write_fn(&mut out, f);
                prev_was_import = false;
            }
            Item::Struct(s) => {
                if !first {
                    out.push('\n');
                }
                write_struct(&mut out, s);
                prev_was_import = false;
            }
            Item::ExternFn { name, params, ret, header, .. } => {
                if !first {
                    out.push('\n');
                }
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.ty))
                    .collect();
                out.push_str(&format!("extern fn {}({})", name, ps.join(", ")));
                if let Some(r) = ret {
                    out.push_str(&format!(" -> {}", r));
                }
                out.push_str(&format!(" from \"{}\"\n", escape_str(header)));
                prev_was_import = false;
            }
            Item::Comment(text) => {
                if !first && !prev_was_import {
                    out.push('\n');
                }
                out.push_str(&format!("# {}\n", text));
                prev_was_import = false;
            }
        }
        first = false;
    }
    out
}

fn write_fn(out: &mut String, f: &FnDecl) {
    let kw = if f.exported { "export " } else { "" };
    let name = match &f.receiver {
        Some(r) => format!("{}.{}", r, f.name),
        None => f.name.clone(),
    };
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| {
            if p.is_self {
                "self".to_string()
            } else {
                format!("{}: {}", p.name, p.ty)
            }
        })
        .collect();
    out.push_str(&format!("{}fn {}({})", kw, name, params.join(", ")));
    if let Some(r) = &f.ret {
        out.push_str(&format!(" -> {}", r));
    }
    out.push_str(":\n");
    write_block(out, &f.body, 1);
}

fn write_struct(out: &mut String, s: &StructDecl) {
    let kw = if s.exported { "export " } else { "" };
    out.push_str(&format!("{}struct {}:\n", kw, s.name));
    for field in &s.fields {
        out.push_str(&format!("  {}: {}", field.name, field.ty));
        if let Some(c) = &field.comment {
            out.push_str(&format!("  # {}", c));
        }
        out.push('\n');
    }
}

fn write_block(out: &mut String, stmts: &[Stmt], depth: usize) {
    let pad = "  ".repeat(depth);
    for st in stmts {
        write_stmt(out, st, &pad);
    }
}

fn write_stmt(out: &mut String, st: &Stmt, pad: &str) {
    match &st.kind {
        StmtKind::Comment(text) => out.push_str(&format!("{}# {}\n", pad, text)),
        StmtKind::Let { binders, ann, value } => {
            out.push_str(&format!("{}let {}", pad, binders.join(", ")));
            if let Some(a) = ann {
                out.push_str(&format!(": {}", a));
            }
            out.push_str(" = ");
            write_expr(out, value, 0);
            out.push('\n');
        }
        StmtKind::AssignMulti { targets, value } => {
            let ts: Vec<String> = targets
                .iter()
                .map(|t| {
                    let mut out = String::new();
                    write_lvalue(&mut out, t);
                    out
                })
                .collect();
            out.push_str(&format!("{}{} = ", pad, ts.join(", ")));
            write_expr(out, value, 0);
            out.push('\n');
        }
        StmtKind::TypedAssign { name, ann, value } => {
            out.push_str(&format!("{}{}: {} = ", pad, name, ann));
            write_expr(out, value, 0);
            out.push('\n');
        }
        StmtKind::Assign { target, value } => {
            out.push_str(pad);
            write_lvalue(out, target);
            out.push_str(" = ");
            write_expr(out, value, 0);
            out.push('\n');
        }
        StmtKind::Return(value) => match value {
            Some(e) => {
                out.push_str(&format!("{}return ", pad));
                write_expr(out, e, 0);
                out.push('\n');
            }
            None => out.push_str(&format!("{}return\n", pad)),
        },
        StmtKind::If { cond, then, elifs, els } => {
            out.push_str(&format!("{}if ", pad));
            write_expr(out, cond, 0);
            out.push_str(":\n");
            write_block(out, then, (pad.len() / 2) + 1);
            for (c, b) in elifs {
                out.push_str(&format!("{}elif ", pad));
                write_expr(out, c, 0);
                out.push_str(":\n");
                write_block(out, b, (pad.len() / 2) + 1);
            }
            if let Some(b) = els {
                out.push_str(&format!("{}else:\n", pad));
                write_block(out, b, (pad.len() / 2) + 1);
            }
        }
        StmtKind::While { cond, body } => {
            out.push_str(&format!("{}while ", pad));
            write_expr(out, cond, 0);
            out.push_str(":\n");
            write_block(out, body, (pad.len() / 2) + 1);
        }
        StmtKind::For { binders, iter, body } => {
            out.push_str(&format!("{}for {} in ", pad, binders.join(", ")));
            match iter {
                Iter::Expr(e) => write_expr(out, e, 0),
                Iter::Range { start, end, inclusive } => {
                    write_expr(out, start, 0);
                    out.push_str(if *inclusive { "..=" } else { ".." });
                    write_expr(out, end, 0);
                }
            }
            out.push_str(":\n");
            write_block(out, body, (pad.len() / 2) + 1);
        }
        StmtKind::Try { body, binder, handler } => {
            out.push_str(&format!("{}try:\n", pad));
            write_block(out, body, (pad.len() / 2) + 1);
            match binder {
                Some(b) => out.push_str(&format!("{}except {}:\n", pad, b)),
                None => out.push_str(&format!("{}except:\n", pad)),
            }
            write_block(out, handler, (pad.len() / 2) + 1);
        }
        StmtKind::Raise(value) => {
            out.push_str(&format!("{}raise ", pad));
            write_expr(out, value, 0);
            out.push('\n');
        }
        StmtKind::Asm(code) => out.push_str(&format!("{}asm(\"{}\")\n", pad, escape_str(code))),
        StmtKind::Break => out.push_str(&format!("{}break\n", pad)),
        StmtKind::Continue => out.push_str(&format!("{}continue\n", pad)),
        StmtKind::Pass => out.push_str(&format!("{}pass\n", pad)),
        StmtKind::Expr(e) => {
            out.push_str(pad);
            write_expr(out, e, 0);
            out.push('\n');
        }
    }
}

fn write_lvalue(out: &mut String, lv: &LValue) {
    match lv {
        LValue::Ident(n) => out.push_str(n),
        LValue::Index(recv, idx) => {
            write_lvalue(out, recv);
            out.push('[');
            write_expr(out, idx, 0);
            out.push(']');
        }
        LValue::Field(recv, name) => {
            write_lvalue(out, recv);
            out.push('.');
            out.push_str(name);
        }
    }
}

fn prec(e: &ExprKind) -> u8 {
    match e {
        ExprKind::Or(..) => 1,
        ExprKind::And(..) => 2,
        ExprKind::Not(_) => 3,
        ExprKind::Compare(..) => 4,
        ExprKind::Binary(op, _, _) => match op {
            BinOp::BitOr => 2,
            BinOp::BitXor => 3,
            BinOp::BitAnd => 4,
            BinOp::Add | BinOp::Sub => 5,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 6,
            BinOp::Pow => 8,
            BinOp::Shl | BinOp::Shr => 5,
        },
        ExprKind::Unary(..) => 7,
        ExprKind::Call(..)
        | ExprKind::Method(..)
        | ExprKind::Field(..)
        | ExprKind::Index(..)
        | ExprKind::Slice(..)
        | ExprKind::Cast(..) => 9,
        _ => 10,
    }
}

pub fn write_expr(out: &mut String, e: &Expr, min: u8) {
    let p = prec(&e.kind);
    let need_paren = p < min;
    if need_paren {
        out.push('(');
    }
    match &e.kind {
        ExprKind::Int(v) => out.push_str(&v.to_string()),
        ExprKind::Float(v) => out.push_str(&fmt_float(*v)),
        ExprKind::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        ExprKind::Nil => out.push_str("nil"),
        ExprKind::Str(s) => out.push_str(&format!("\"{}\"", escape_str(s))),
        ExprKind::FStr(parts) => {
            out.push_str("f\"");
            for part in parts {
                match part {
                    FStrPartAst::Lit(l) => {
                        let escaped = l
                            .replace('{', "{{")
                            .replace('}', "}}");
                        out.push_str(&escape_str(&escaped));
                    }
                    FStrPartAst::E(sub) => {
                        out.push('{');
                        write_expr(out, sub, 0);
                        out.push('}');
                    }
                }
            }
            out.push('"');
        }
        ExprKind::Ident(n) => out.push_str(n),
        ExprKind::Unary(_, inner) => {
            out.push('-');
            write_expr(out, inner, 7);
        }
        ExprKind::Binary(op, l, r) => {
            let (lp, rp) = match op {
                BinOp::Pow => (9, 8),  // right associative
                _ => (p, p + 1),        // left associative
            };
            write_expr(out, l, lp);
            out.push_str(&format!(" {} ", op_str(op)));
            write_expr(out, r, rp);
        }
        ExprKind::And(l, r) => {
            write_expr(out, l, 2);
            out.push_str(" and ");
            write_expr(out, r, 3);
        }
        ExprKind::Or(l, r) => {
            write_expr(out, l, 1);
            out.push_str(" or ");
            write_expr(out, r, 2);
        }
        ExprKind::Not(inner) => {
            out.push_str("not ");
            write_expr(out, inner, 3);
        }
        ExprKind::Compare(op, l, r) => {
            write_expr(out, l, 5);
            out.push_str(&format!(" {} ", cmp_str(op)));
            write_expr(out, r, 5);
        }
        ExprKind::Call(callee, args) => {
            write_expr(out, callee, 9);
            out.push('(');
            write_args(out, args);
            out.push(')');
        }
        ExprKind::Method(recv, name, args) => {
            write_expr(out, recv, 9);
            out.push('.');
            out.push_str(name);
            out.push('(');
            write_args(out, args);
            out.push(')');
        }
        ExprKind::Field(recv, name) => {
            write_expr(out, recv, 9);
            out.push('.');
            out.push_str(name);
        }
        ExprKind::Index(recv, idx) => {
            write_expr(out, recv, 9);
            out.push('[');
            write_expr(out, idx, 0);
            out.push(']');
        }
        ExprKind::Slice(recv, start, end) => {
            write_expr(out, recv, 9);
            out.push('[');
            if let Some(s) = start {
                write_expr(out, s, 0);
            }
            out.push(':');
            if let Some(e) = end {
                write_expr(out, e, 0);
            }
            out.push(']');
        }
        ExprKind::ListLit(items) => {
            out.push('[');
            write_args(out, items);
            out.push(']');
        }
        ExprKind::DictLit(pairs) => {
            out.push('{');
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_expr(out, k, 0);
                out.push_str(": ");
                write_expr(out, v, 0);
            }
            out.push('}');
        }
        ExprKind::TupleLit(a, b) => {
            out.push('(');
            write_expr(out, a, 0);
            out.push_str(", ");
            write_expr(out, b, 0);
            out.push(')');
        }
        ExprKind::FnExpr(f) => {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| match &p.default {
                    Some(d) => {
                        let mut out = format!("{}: {} = ", p.name, p.ty);
                        write_expr(&mut out, d, 0);
                        out
                    }
                    None => format!("{}: {}", p.name, p.ty),
                })
                .collect();
            out.push_str(&format!("fn({})", params.join(", ")));
            if let Some(r) = &f.ret {
                out.push_str(&format!(" -> {}", r));
            }
            out.push(':');
            let mut body = String::new();
            write_block(&mut body, &f.body, 1);
            out.push_str(&body);
        }
        ExprKind::Comprehension { item, binders, iter, cond } => {
            out.push('[');
            write_expr(out, item, 0);
            out.push_str(&format!(" for {} in ", binders.join(", ")));
            match &**iter {
                Iter::Expr(e) => write_expr(out, e, 0),
                Iter::Range { start, end, inclusive } => {
                    write_expr(out, start, 0);
                    out.push_str(if *inclusive { "..=" } else { ".." });
                    write_expr(out, end, 0);
                }
            }
            if let Some(c) = cond {
                out.push_str(" if ");
                write_expr(out, c, 0);
            }
            out.push(']');
        }
        ExprKind::Cast(t, inner) => {
            out.push_str(&format!("cast[{}](", t));
            write_expr(out, inner, 0);
            out.push(')');
        }
    }
    if need_paren {
        out.push(')');
    }
}

fn write_args(out: &mut String, args: &[Expr]) {
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write_expr(out, a, 0);
    }
}

fn op_str(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

fn cmp_str(op: &CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::Ne => "!=",
        CmpOp::Lt => "<",
        CmpOp::Gt => ">",
        CmpOp::Le => "<=",
        CmpOp::Ge => ">=",
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\0', "\\0")
        .replace('"', "\\\"")
}

/// Shortest float rendering that round-trips.
pub fn fmt_float(v: f64) -> String {
    if v == v.trunc() && v.is_finite() && v.abs() < 1e15 {
        // Keep a trailing .0 so the literal stays a float on re-parse.
        format!("{:.1}", v)
    } else {
        for prec in 1..=17 {
            let s = format!("{:.*}", prec, v);
            if s.parse::<f64>() == Ok(v) {
                return s;
            }
        }
        format!("{}", v)
    }
}
