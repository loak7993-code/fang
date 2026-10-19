//! Semantic analysis: name resolution, per-function type inference,
//! method sugar, and monomorphization records.

use crate::ast::*;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    Ptr,
    List(Box<Ty>),
    Dict(Box<Ty>, Box<Ty>),
    Tuple(Box<Ty>, Box<Ty>),
    Struct(String),
    Func(String),
    /// The except binder: printable and re-raisable, nothing else.
    Any,
    Void,
    Err,
}

impl Ty {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }
    pub fn printable(&self) -> bool {
        matches!(
            self,
            Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Any | Ty::Ptr
        )
    }
    /// Type name for error messages.
    pub fn show(&self, a: &Analysis) -> String {
        match self {
            Ty::Int => "int".into(),
            Ty::Float => "float".into(),
            Ty::Bool => "bool".into(),
            Ty::Str => "str".into(),
            Ty::Ptr => "ptr".into(),
            Ty::List(t) => format!("list[{}]", t.show(a)),
            Ty::Dict(k, v) => format!("dict[{}, {}]", k.show(a), v.show(a)),
            Ty::Tuple(x, y) => format!("({}, {})", x.show(a), y.show(a)),
            Ty::Struct(m) => a
                .structs
                .get(m)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| m.clone()),
            Ty::Func(_) => "fn".into(),
            Ty::Any => "any (except value)".into(),
            Ty::Void => "void".into(),
            Ty::Err => "unknown".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SemaError {
    pub msg: String,
    pub line: u32,
    pub col: u32,
    pub module: String,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub ty: Ty,
    pub is_self: bool,
}

#[derive(Debug, Clone)]
pub struct FnInfo {
    pub mangled: String,
    /// Raw C symbol for externs; equals mangled for Leo functions.
    pub c_name: String,
    pub module: String,
    pub name: String,
    pub params: Vec<ParamInfo>,
    pub ret: Ty,
    pub exported: bool,
    pub is_extern: bool,
    pub header: String,
    pub receiver: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub mangled: String,
    pub module: String,
    pub name: String,
    pub fields: Vec<(String, Ty)>,
    pub methods: HashMap<String, String>,
    pub exported: bool,
}

#[derive(Debug, Clone)]
pub enum ConstVal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListOp {
    Push,
    Pop,
    Insert,
    RemoveAt,
    Sort,
    Reverse,
    Contains,
    IndexOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictOp {
    Get,
    Set,
    Keys,
    Values,
    Items,
    Contains,
    Remove,
}

#[derive(Debug, Clone)]
pub enum CallTarget {
    /// Mangled function; call with explicit args only.
    Fn(String),
    /// Mangled function; receiver is passed as an implicit first argument.
    FnSelf(String),
    StructNew(String),
    BuiltinPrint,
    BuiltinLen,
    BuiltinAssert,
    ListMethod(ListOp),
    DictMethod(DictOp),
    /// s.to_cstr() -> ptr
    ToCStr,
    /// n.to_str() -> str on int/float/bool
    ToStr,
    /// float(int)
    BuiltinFloat,
    /// int(float)
    BuiltinInt,
}

pub struct Analysis {
    pub structs: HashMap<String, StructInfo>,
    pub fns: HashMap<String, FnInfo>,
    pub consts: HashMap<String, ConstVal>,
    /// (module, expr span start) -> inferred type.
    pub ty: HashMap<(String, u32), Ty>,
    /// (module, expr span start) -> resolved call target.
    pub call: HashMap<(String, u32), CallTarget>,
    /// (module, stmt span start) -> for-binder types.
    pub for_binders: HashMap<(String, u32), Vec<Ty>>,
    /// (module, stmt span start) -> except binder type.
    pub catch_ty: HashMap<(String, u32), Ty>,
    /// Collection instantiations the codegen must stamp.
    pub insts: BTreeSet<Ty>,
    pub errors: Vec<SemaError>,
    /// Functions currently being inferred (self-recursion detection).
    pub in_progress: HashSet<String>,
    /// Module name -> imported module names (resolved).
    pub imports: HashMap<String, Vec<String>>,
    pub modules: Vec<String>,
}

impl Analysis {
    fn err(&mut self, msg: impl Into<String>, span: Span, module: &str) {
        if self.errors.len() < 50 {
            self.errors.push(SemaError {
                msg: msg.into(),
                line: span.line,
                col: span.col,
                module: module.to_string(),
            });
        }
    }
}

struct Ctx {
    scopes: Vec<HashMap<String, Ty>>,
    ret: Ty,
    loop_depth: u32,
}

impl Ctx {
    fn lookup(&self, name: &str) -> Option<Ty> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }
    fn declare(&mut self, name: &str, ty: Ty) -> bool {
        self.scopes.last_mut().unwrap().insert(name.to_string(), ty).is_none()
    }
}

pub fn analyze(sources: Vec<(String, Module)>, entry: &str) -> Analysis {
    let mut a = Analysis {
        structs: HashMap::new(),
        fns: HashMap::new(),
        consts: HashMap::new(),
        ty: HashMap::new(),
        call: HashMap::new(),
        for_binders: HashMap::new(),
        catch_ty: HashMap::new(),
        insts: BTreeSet::new(),
        errors: Vec::new(),
        in_progress: HashSet::new(),
        imports: HashMap::new(),
        modules: Vec::new(),
    };

    // ---- pass 1: register declarations ----
    for (name, _m) in &sources {
        a.modules.push(name.clone());
    }
    let module_names: HashSet<String> = a.modules.iter().cloned().collect();

    for (mname, m) in &sources {
        for item in &m.items {
            match item {
                Item::Struct(s) => {
                    let mangled = format!("s_{}_{}", mname, s.name);
                    if a.structs.contains_key(&mangled) {
                        a.err(format!("struct '{}' is defined twice", s.name), s.span, mname);
                        continue;
                    }
                    let prev = a
                        .structs
                        .values()
                        .any(|x| x.module == *mname && x.name == s.name);
                    if prev {
                        a.err(format!("struct '{}' is defined twice", s.name), s.span, mname);
                        continue;
                    }
                    a.structs.insert(
                        mangled.clone(),
                        StructInfo {
                            mangled,
                            module: mname.clone(),
                            name: s.name.clone(),
                            fields: Vec::new(), // filled in pass 1b
                            methods: HashMap::new(),
                            exported: s.exported,
                        },
                    );
                }
                Item::Fn(f) => {
                    let mangled = mangle_fn(mname, f);
                    if a.fns.contains_key(&mangled) {
                        a.err(format!("'{}' is defined twice", f.name), f.span, mname);
                        continue;
                    }
                    a.fns.insert(
                        mangled.clone(),
                        FnInfo {
                            mangled,
                            c_name: String::new(),
                            module: mname.clone(),
                            name: f.name.clone(),
                            params: Vec::new(),
                            ret: Ty::Err,
                            exported: f.exported,
                            is_extern: false,
                            header: String::new(),
                            receiver: f.receiver.clone(),
                        },
                    );
                }
                Item::ExternFn { name, header, span, .. } => {
                    let mangled = format!("x_{}_{}", mname, name);
                    if a.fns.contains_key(&mangled) {
                        a.err(format!("extern fn '{}' is declared twice", name), *span, mname);
                        continue;
                    }
                    a.fns.insert(
                        mangled.clone(),
                        FnInfo {
                            mangled,
                            c_name: name.clone(),
                            module: mname.clone(),
                            name: name.clone(),
                            params: Vec::new(),
                            ret: Ty::Err,
                            exported: true,
                            is_extern: true,
                            header: header.clone(),
                            receiver: None,
                        },
                    );
                }
                Item::Const { .. } | Item::Comment(_) | Item::Import { .. } => {}
            }
        }
    }

    // ---- pass 1b: resolve imports ----
    for (mname, m) in &sources {
        let mut imps = Vec::new();
        for item in &m.items {
            if let Item::Import { module, span, .. } = item {
                if module_names.contains(module) {
                    imps.push(module.clone());
                } else {
                    a.err(
                        format!("unknown module '{}'; not a std module, local file, or package", module),
                        *span,
                        mname,
                    );
                }
            }
        }
        a.imports.insert(mname.clone(), imps);
    }

    // ---- pass 2: struct fields, fn params, methods ----
    for (mname, m) in &sources {
        for item in &m.items {
            match item {
                Item::Struct(s) => {
                    let mangled = format!("s_{}_{}", mname, s.name);
                    let mut fields = Vec::new();
                    for f in &s.fields {
                        let ty = conv_type(&f.ty, mname, &a);
                        if matches!(ty, Ty::Err) {
                            a.err(
                                format!("unknown type '{}' in field '{}'", f.ty, f.name),
                                s.span,
                                mname,
                            );
                        }
                        fields.push((f.name.clone(), ty));
                    }
                    if let Some(si) = a.structs.get_mut(&mangled) {
                        si.fields = fields;
                    }
                }
                Item::Fn(f) => {
                    let mangled = mangle_fn(mname, f);
                    let mut params = Vec::new();
                    for p in &f.params {
                        let ty = if p.is_self {
                            Ty::Struct(format!("s_{}_{}", mname, f.receiver.clone().unwrap_or_default()))
                        } else {
                            conv_type(&p.ty, mname, &a)
                        };
                        params.push(ParamInfo { name: p.name.clone(), ty, is_self: p.is_self });
                    }
                    let declared = f.ret.as_ref().map(|r| conv_type(r, mname, &a));
                    if let Some(fi) = a.fns.get_mut(&mangled) {
                        fi.params = params;
                        fi.ret = declared.clone().unwrap_or(Ty::Err);
                    }
                    // receiver must exist; method names must not collide with fields
                    if let Some(recv) = &f.receiver {
                        let sm = format!("s_{}_{}", mname, recv);
                        let Some(si) = a.structs.get(&sm) else {
                            a.err(
                                format!("method '{}' refers to unknown struct '{}'", f.name, recv),
                                f.span,
                                mname,
                            );
                            continue;
                        };
                        if si.fields.iter().any(|(n, _)| *n == f.name) {
                            a.err(
                                format!("struct '{}' already has a field named '{}'", recv, f.name),
                                f.span,
                                mname,
                            );
                            continue;
                        }
                    }
                    let _ = declared;
                }
                Item::ExternFn { name, params, ret, span, .. } => {
                    let mangled = format!("x_{}_{}", mname, name);
                    let mut pis = Vec::new();
                    for p in params {
                        let ty = conv_type(&p.ty, mname, &a);
                        pis.push(ParamInfo { name: p.name.clone(), ty, is_self: false });
                    }
                    let r = ret.as_ref().map(|t| conv_type(t, mname, &a)).unwrap_or(Ty::Void);
                    if let Some(fi) = a.fns.get_mut(&mangled) {
                        fi.params = pis;
                        fi.ret = r;
                    }
                    let _ = span;
                }
                _ => {}
            }
        }
    }

    // register methods on structs
    for (mname, m) in &sources {
        for item in &m.items {
            if let Item::Fn(f) = item {
                if let Some(recv) = &f.receiver {
                    let sm = format!("s_{}_{}", mname, recv);
                    let fm = mangle_fn(mname, f);
                    if let Some(si) = a.structs.get_mut(&sm) {
                        si.methods.insert(f.name.clone(), fm);
                    }
                }
            }
        }
    }

    // struct recursion check: a struct containing itself by value is a
    // compile error (it would be infinitely sized in C).
    let mut cyclic = Vec::new();
    for mangled in a.structs.keys().cloned().collect::<Vec<_>>() {
        let mut path = Vec::new();
        if contains_struct_cycle(&mangled, &a.structs, &mut path) {
            cyclic.push(mangled);
        }
    }
    for mangled in cyclic {
        let (mn, nm) = {
            let si = &a.structs[&mangled];
            (si.module.clone(), si.name.clone())
        };
        a.errors.push(SemaError {
            msg: format!("struct '{}' contains itself by value; use a list field or restructure", nm),
            line: 0,
            col: 0,
            module: mn,
        });
    }

    // ---- pass 3: infer fn bodies ----
    for (mname, m) in &sources {
        for item in &m.items {
            if let Item::Fn(f) = item {
                infer_fn(f, mname, &mut a);
            }
            if let Item::Const { name, value, span, .. } = item {
                infer_const(name, value, *span, mname, &mut a);
            }
        }
    }

    // ---- entry check ----
    let entry_main = format!("f_{}_main", entry);
    if let Some(fi) = a.fns.get(&entry_main) {
        if !fi.params.is_empty() {
            let (mn, _) = (fi.module.clone(), fi.name.clone());
            a.err("fn main() must take no parameters", Span::new(0, 0, 0, 0), &mn);
        }
    } else {
        a.err(
            "no fn main() in the entry module",
            Span::new(0, 0, 0, 0),
            entry,
        );
    }

    a
}

fn mangle_fn(module: &str, f: &FnDecl) -> String {
    match &f.receiver {
        Some(r) => format!("f_{}_{}_{}", module, r, f.name),
        None => format!("f_{}_{}", module, f.name),
    }
}

fn contains_struct_cycle(
    mangled: &str,
    structs: &HashMap<String, StructInfo>,
    path: &mut Vec<String>,
) -> bool {
    if path.iter().any(|m| m == mangled) {
        return true;
    }
    path.push(mangled.to_string());
    let mut found = false;
    if let Some(si) = structs.get(mangled) {
        for (_, t) in &si.fields {
            if let Ty::Struct(inner) = t {
                if contains_struct_cycle(inner, structs, path) {
                    found = true;
                    break;
                }
            }
        }
    }
    path.pop();
    found
}

/// Convert an AST type annotation into a semantic type.
fn conv_type(t: &Type, module: &str, a: &Analysis) -> Ty {
    match t {
        Type::Int => Ty::Int,
        Type::Float => Ty::Float,
        Type::Bool => Ty::Bool,
        Type::Str => Ty::Str,
        Type::Ptr => Ty::Ptr,
        Type::List(e) => Ty::List(Box::new(conv_type(e, module, a))),
        Type::Dict(k, v) => Ty::Dict(Box::new(conv_type(k, module, a)), Box::new(conv_type(v, module, a))),
        Type::Tuple(x, y) => Ty::Tuple(Box::new(conv_type(x, module, a)), Box::new(conv_type(y, module, a))),
        Type::Named(n) => {
            let mangled = format!("s_{}_{}", module, n);
            if a.structs.contains_key(&mangled) {
                Ty::Struct(mangled)
            } else {
                Ty::Err
            }
        }
    }
}

fn infer_const(name: &str, value: &Expr, span: Span, module: &str, a: &mut Analysis) {
    let t = infer_expr(value, module, a, &mut Ctx {
        scopes: vec![HashMap::new()],
        ret: Ty::Void,
        loop_depth: 0,
    });
    let _ = t;
    match const_eval(value) {
        Some(v) => {
            a.consts.insert(format!("c_{}_{}", module, name), v);
        }
        None => {
            a.err(
                format!("top-level let '{}' must be a constant expression (literals and literal arithmetic)", name),
                span,
                module,
            );
        }
    }
}

fn const_eval(e: &Expr) -> Option<ConstVal> {
    Some(match &e.kind {
        ExprKind::Int(v) => ConstVal::Int(*v),
        ExprKind::Float(v) => ConstVal::Float(*v),
        ExprKind::Bool(v) => ConstVal::Bool(*v),
        ExprKind::Str(s) => ConstVal::Str(s.clone()),
        ExprKind::Unary(BinOp::Sub, inner) => match const_eval(inner)? {
            ConstVal::Int(v) => ConstVal::Int(-v),
            ConstVal::Float(v) => ConstVal::Float(-v),
            _ => return None,
        },
        ExprKind::Binary(op, l, r) => {
            let lv = const_eval(l)?;
            let rv = const_eval(r)?;
            match (lv, rv) {
                (ConstVal::Int(x), ConstVal::Int(y)) => ConstVal::Int(match op {
                    BinOp::Add => x.checked_add(y)?,
                    BinOp::Sub => x.checked_sub(y)?,
                    BinOp::Mul => x.checked_mul(y)?,
                    BinOp::Div => x.checked_div(y)?,
                    BinOp::Mod => x.checked_rem(y)?,
                    BinOp::Pow => return None,
                }),
                (ConstVal::Float(x), ConstVal::Float(y)) => ConstVal::Float(match op {
                    BinOp::Add => x + y,
                    BinOp::Sub => x - y,
                    BinOp::Mul => x * y,
                    BinOp::Div => x / y,
                    _ => return None,
                }),
                (ConstVal::Str(x), ConstVal::Str(y)) if *op == BinOp::Add => ConstVal::Str(x + &y),
                _ => return None,
            }
        }
        _ => return None,
    })
}

fn infer_fn(f: &FnDecl, module: &str, a: &mut Analysis) {
    let mangled = mangle_fn(module, f);
    let mut scope = HashMap::new();
    let fi = a.fns.get(&mangled).unwrap();
    for p in &fi.params {
        scope.insert(p.name.clone(), p.ty.clone());
    }
    let declared = fi.ret.clone();
    let is_void_declared = f.ret.is_none();

    let mut ctx = Ctx {
        scopes: vec![scope],
        ret: declared.clone(),
        loop_depth: 0,
    };
    a.in_progress.insert(mangled.clone());
    let inferred = infer_block(&f.body, module, a, &mut ctx, declared, is_void_declared);
    a.in_progress.remove(&mangled);

    if let Some(fi) = a.fns.get_mut(&mangled) {
        fi.ret = inferred;
    }
}

/// Return-flow summary for a block.
#[derive(Clone)]
struct RetInfo {
    /// Type of some return seen anywhere in the block (for conflicts).
    seen: Option<Ty>,
    /// True when every path through the block returns.
    guaranteed: bool,
}

impl RetInfo {
    fn falls() -> Self {
        RetInfo { seen: None, guaranteed: false }
    }
    fn merge_seen(&mut self, t: Ty, span: Span, module: &str, a: &mut Analysis) {
        match &self.seen {
            None => self.seen = Some(t),
            Some(prev) => {
                if *prev != t && *prev != Ty::Err && t != Ty::Err {
                    a.errors.push(SemaError {
                        msg: format!(
                            "return type mismatch: earlier returns give {}, this returns {}",
                            prev.show(a),
                            t.show(a)
                        ),
                        line: span.line,
                        col: span.col,
                        module: module.to_string(),
                    });
                }
            }
        }
    }
}

fn block_ret_info(
    stmts: &[Stmt],
    module: &str,
    a: &mut Analysis,
    ctx: &mut Ctx,
) -> RetInfo {
    ctx.scopes.push(HashMap::new());
    let mut info = RetInfo::falls();
    for st in stmts {
        if info.guaranteed {
            break; // unreachable code; types already decided
        }
        let r = infer_stmt(st, module, a, ctx);
        if let Some(t) = r {
            info.merge_seen(t, st.span, module, a);
        }
        if stmt_guarantees_return(st, module, a, ctx) {
            info.guaranteed = true;
        }
    }
    ctx.scopes.pop();
    info
}

fn stmt_guarantees_return(st: &Stmt, module: &str, a: &mut Analysis, ctx: &mut Ctx) -> bool {
    match &st.kind {
        StmtKind::Return(_) => true,
        // raise never falls through: the statement unwinds.
        StmtKind::Raise(_) => true,
        StmtKind::If { then, elifs, els, .. } => {
            let Some(els) = els else { return false };
            if !block_guarantees(then, module, a, ctx) {
                return false;
            }
            for (_, b) in elifs {
                if !block_guarantees(b, module, a, ctx) {
                    return false;
                }
            }
            block_guarantees(els, module, a, ctx)
        }
        _ => false,
    }
}

fn block_guarantees(stmts: &[Stmt], module: &str, a: &mut Analysis, ctx: &mut Ctx) -> bool {
    let last = match stmts.last() {
        Some(l) => l,
        None => return false,
    };
    stmt_guarantees_return(last, module, a, ctx)
}

fn infer_block(
    stmts: &[Stmt],
    module: &str,
    a: &mut Analysis,
    ctx: &mut Ctx,
    declared: Ty,
    declared_is_none: bool,
) -> Ty {
    let info = block_ret_info(stmts, module, a, ctx);
    if info.seen.is_some() && !info.guaranteed {
        a.errors.push(SemaError {
            msg: "not every path returns a value; add a return at the end or an else branch"
                .to_string(),
            line: 0,
            col: 0,
            module: module.to_string(),
        });
    }
    let inferred = info.seen.unwrap_or(Ty::Void);
    if !declared_is_none && declared != Ty::Err && inferred != declared {
        a.errors.push(SemaError {
            msg: format!(
                "declared return type {} does not match inferred {}",
                declared.show(a),
                inferred.show(a)
            ),
            line: 0,
            col: 0,
            module: module.to_string(),
        });
    }
    inferred
}

fn infer_stmt(st: &Stmt, module: &str, a: &mut Analysis, ctx: &mut Ctx) -> Option<Ty> {
    match &st.kind {
        StmtKind::Comment(_) => None,
        StmtKind::Pass => None,
        StmtKind::Asm(_) => None,
        StmtKind::Break | StmtKind::Continue => {
            if ctx.loop_depth == 0 {
                a.err(
                    format!("'{}' outside a loop", if matches!(st.kind, StmtKind::Break) { "break" } else { "continue" }),
                    st.span,
                    module,
                );
            }
            None
        }
        StmtKind::Let { binders, ann, value } => {
            let expected = ann.as_ref().map(|t| conv_type(t, module, a));
            // An annotation types an empty collection literal directly.
            let vt = match (&value.kind, &expected) {
                (ExprKind::ListLit(items), Some(exp @ Ty::List(_))) if items.is_empty() => {
                    a.ty.insert((module.to_string(), value.span.start), exp.clone());
                    exp.clone()
                }
                (ExprKind::DictLit(pairs), Some(exp @ Ty::Dict(_, _))) if pairs.is_empty() => {
                    a.ty.insert((module.to_string(), value.span.start), exp.clone());
                    exp.clone()
                }
                _ => infer_expr(value, module, a, ctx),
            };
            match expected {
                Some(exp) => {
                    if exp != vt && vt != Ty::Err {
                        a.err(
                            format!(
                                "annotation {} does not match the value {}",
                                exp.show(a),
                                vt.show(a)
                            ),
                            st.span,
                            module,
                        );
                    }
                    if binders.len() == 2 {
                        let (x, y) = split_tuple(&exp, a);
                        ctx.declare(&binders[0], x);
                        ctx.declare(&binders[1], y);
                    } else {
                        ctx.declare(&binders[0], exp);
                    }
                }
                None => match vt {
                    Ty::Tuple(x, y) if binders.len() == 2 => {
                        let (x, y) = (x.as_ref().clone(), y.as_ref().clone());
                        ctx.declare(&binders[0], x);
                        ctx.declare(&binders[1], y);
                    }
                    Ty::Tuple(..) if binders.len() == 1 => {
                        a.err(
                            "destructuring a tuple needs two names: let a, b = ...".to_string(),
                            st.span,
                            module,
                        );
                        ctx.declare(&binders[0], Ty::Err);
                    }
                    t if binders.len() == 2 => {
                        a.err(
                            format!(
                                "two binders need a tuple value, got {}; to loop a dict use for k, v in d.items()",
                                t.show(a)
                            ),
                            st.span,
                            module,
                        );
                        ctx.declare(&binders[0], Ty::Err);
                        ctx.declare(&binders[1], Ty::Err);
                    }
                    t => {
                        ctx.declare(&binders[0], t);
                    }
                },
            }
            None
        }
        StmtKind::Assign { target, value } => {
            let vt = infer_expr(value, module, a, ctx);
            match target {
                LValue::Ident(n) => match ctx.lookup(n) {
                    Some(prev) => {
                        if prev != vt && vt != Ty::Err && prev != Ty::Err {
                            a.err(
                                format!(
                                    "'{}' is {}; assigning {} is not allowed (convert the value or use a new let)",
                                    n,
                                    prev.show(a),
                                    vt.show(a)
                                ),
                                st.span,
                                module,
                            );
                        }
                    }
                    None => {
                        a.err(
                            format!("name '{}' is not declared; write let {} = ... first", n, n),
                            st.span,
                            module,
                        );
                    }
                },
                LValue::Index(recv, idx) => {
                    let rt = infer_lvalue_ty(recv, module, a, ctx);
                    let it = infer_expr(idx, module, a, ctx);
                    check_index_set(&rt, &it, &vt, st.span, module, a);
                }
                LValue::Field(recv, name) => {
                    let rt = infer_lvalue_ty(recv, module, a, ctx);
                    if let Ty::Struct(m) = rt {
                        if let Some(si) = a.structs.get(&m) {
                            match si.fields.iter().find(|(n, _)| n == name) {
                                Some((_, ft)) => {
                                    if *ft != vt && vt != Ty::Err {
                                        a.err(
                                            format!(
                                                "field '{}': expected {}, got {}",
                                                name,
                                                ft.show(a),
                                                vt.show(a)
                                            ),
                                            st.span,
                                            module,
                                        );
                                    }
                                }
                                None => {
                                    let names: Vec<&str> =
                                        si.fields.iter().map(|(n, _)| n.as_str()).collect();
                                    a.err(
                                        format!(
                                            "struct '{}' has no field '{}'; {}",
                                            si.name,
                                            name,
                                            did_you_mean(name, &names)
                                        ),
                                        st.span,
                                        module,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        StmtKind::Return(value) => {
            let t = match value {
                Some(e) => infer_expr(e, module, a, ctx),
                None => Ty::Void,
            };
            Some(t)
        }
        StmtKind::If { cond, then, elifs, els } => {
            let ct = infer_expr(cond, module, a, ctx);
            if ct != Ty::Bool {
                a.err(
                    format!("if condition must be bool, got {}", ct.show(a)),
                    st.span,
                    module,
                );
            }
            infer_block(then, module, a, ctx, ctx.ret.clone(), true);
            for (c, b) in elifs {
                let ct = infer_expr(c, module, a, ctx);
                if ct != Ty::Bool {
                    a.err(format!("elif condition must be bool, got {}", ct.show(a)), st.span, module);
                }
                infer_block(b, module, a, ctx, ctx.ret.clone(), true);
            }
            if let Some(b) = els {
                infer_block(b, module, a, ctx, ctx.ret.clone(), true);
            }
            // Branch returns participate in conflict detection.
            let mut seen: Option<Ty> = None;
            let mut collect = |b: &Block, seen: &mut Option<Ty>| {
                let sub = block_ret_info(b, module, a, ctx);
                if let Some(t) = sub.seen {
                    match seen {
                        None => *seen = Some(t),
                        Some(prev) => {
                            if *prev != t && *prev != Ty::Err && t != Ty::Err {
                                a.errors.push(SemaError {
                                    msg: format!(
                                        "return type mismatch: {} then {}",
                                        prev.show(a),
                                        t.show(a)
                                    ),
                                    line: st.span.line,
                                    col: st.span.col,
                                    module: module.to_string(),
                                });
                            }
                        }
                    }
                }
            };
            collect(then, &mut seen);
            for (_, b) in elifs {
                collect(b, &mut seen);
            }
            if let Some(b) = els {
                collect(b, &mut seen);
            }
            seen
        }
        StmtKind::While { cond, body } => {
            let ct = infer_expr(cond, module, a, ctx);
            if ct != Ty::Bool {
                a.err(
                    format!("while condition must be bool, got {}", ct.show(a)),
                    st.span,
                    module,
                );
            }
            ctx.loop_depth += 1;
            let sub = block_ret_info(body, module, a, ctx);
            ctx.loop_depth -= 1;
            // A while loop can fall through; it never guarantees a return,
            // but returns inside it still count toward the fn's type.
            sub.seen
        }
        StmtKind::For { binders, iter, body } => {
            let binder_tys: Vec<Ty> = match iter {
                Iter::Range { start, end, .. } => {
                    let stt = infer_expr(start, module, a, ctx);
                    let ent = infer_expr(end, module, a, ctx);
                    if stt != Ty::Int || ent != Ty::Int {
                        a.err(
                            "range bounds must be int".to_string(),
                            st.span,
                            module,
                        );
                    }
                    if binders.len() != 1 {
                        a.err(
                            "a range loop takes one variable: for i in 0..10".to_string(),
                            st.span,
                            module,
                        );
                    }
                    vec![Ty::Int; binders.len().max(1)]
                }
                Iter::Expr(e) => {
                    let it = infer_expr(e, module, a, ctx);
                    match it {
                        Ty::List(t) => {
                            if binders.len() == 2 {
                                if let Ty::Tuple(x, y) = t.as_ref() {
                                    vec![x.as_ref().clone(), y.as_ref().clone()]
                                } else {
                                    a.err(
                                        "two loop variables need pairs, like d.items()".to_string(),
                                        st.span,
                                        module,
                                    );
                                    vec![Ty::Err, Ty::Err]
                                }
                            } else if binders.len() == 1 {
                                vec![t.as_ref().clone()]
                            } else {
                                vec![Ty::Err]
                            }
                        }
                        Ty::Dict(k, _) => {
                            if binders.len() != 1 {
                                a.err(
                                    "iterating a dict yields keys; for k, v needs d.items()".to_string(),
                                    st.span,
                                    module,
                                );
                            }
                            vec![k.as_ref().clone()]
                        }
                        Ty::Err => vec![Ty::Err; binders.len().max(1)],
                        other => {
                            a.err(
                                format!(
                                    "cannot iterate {}; lists, dicts, and d.items() are iterable",
                                    other.show(a)
                                ),
                                st.span,
                                module,
                            );
                            vec![Ty::Err; binders.len().max(1)]
                        }
                    }
                }
            };
            a.for_binders.insert((module.to_string(), st.span.start), binder_tys.clone());
            ctx.scopes.push(HashMap::new());
            for (b, t) in binders.iter().zip(binder_tys.iter()) {
                ctx.declare(b, t.clone());
            }
            ctx.loop_depth += 1;
            let sub = block_ret_info(body, module, a, ctx);
            ctx.loop_depth -= 1;
            ctx.scopes.pop();
            // Loops fall through; they never guarantee a return, but
            // returns inside them still count toward the fn's type.
            sub.seen
        }
        StmtKind::Try { body, binder, handler } => {
            let mut sub = block_ret_info(body, module, a, ctx);
            if let Some(b) = binder {
                a.catch_ty.insert((module.to_string(), st.span.start), Ty::Any);
                ctx.scopes.push(HashMap::new());
                ctx.declare(b, Ty::Any);
                let h = block_ret_info(handler, module, a, ctx);
                if h.seen.is_some() && sub.seen.is_none() {
                    sub.seen = h.seen;
                } else if let (Some(x), Some(y)) = (&sub.seen, &h.seen) {
                    if x != y && *x != Ty::Err && *y != Ty::Err {
                        a.errors.push(SemaError {
                            msg: format!(
                                "return type mismatch: try gives {}, except gives {}",
                                x.show(a),
                                y.show(a)
                            ),
                            line: st.span.line,
                            col: st.span.col,
                            module: module.to_string(),
                        });
                    }
                }
                ctx.scopes.pop();
            } else {
                let h = block_ret_info(handler, module, a, ctx);
                if h.seen.is_some() && sub.seen.is_none() {
                    sub.seen = h.seen;
                }
            }
            sub.seen
        }
        StmtKind::Raise(value) => {
            let t = infer_expr(value, module, a, ctx);
            if !t.printable() {
                a.err(
                    "raise takes a printable value; a str is the usual choice".to_string(),
                    st.span,
                    module,
                );
            }
            None
        }
        StmtKind::Expr(e) => {
            infer_expr(e, module, a, ctx);
            None
        }
    }
}

fn split_tuple(t: &Ty, a: &Analysis) -> (Ty, Ty) {
    match t {
        Ty::Tuple(x, y) => (x.as_ref().clone(), y.as_ref().clone()),
        other => {
            let _ = a;
            (other.clone(), other.clone())
        }
    }
}

fn infer_lvalue_ty(lv: &LValue, module: &str, a: &mut Analysis, ctx: &mut Ctx) -> Ty {
    match lv {
        LValue::Ident(n) => ctx.lookup(n).unwrap_or(Ty::Err),
        LValue::Index(recv, idx) => {
            let rt = infer_lvalue_ty(recv, module, a, ctx);
            let _ = infer_expr(idx, module, a, ctx);
            match rt {
                Ty::List(t) => *t,
                Ty::Dict(_, v) => *v,
                _ => Ty::Err,
            }
        }
        LValue::Field(recv, name) => {
            let rt = infer_lvalue_ty(recv, module, a, ctx);
            if let Ty::Struct(m) = rt {
                if let Some(si) = a.structs.get(&m) {
                    if let Some((_, ft)) = si.fields.iter().find(|(n, _)| n == name) {
                        return ft.clone();
                    }
                }
            }
            Ty::Err
        }
    }
}

fn check_index_set(rt: &Ty, it: &Ty, vt: &Ty, span: Span, module: &str, a: &mut Analysis) {
    match rt {
        Ty::List(elem) => {
            if *it != Ty::Int {
                a.err(format!("list index must be int, got {}", it.show(a)), span, module);
            }
            if **elem != *vt && *vt != Ty::Err {
                a.err(
                    format!("list holds {}, got {}", elem.show(a), vt.show(a)),
                    span,
                    module,
                );
            }
        }
        Ty::Dict(k, v) => {
            if **k != *it {
                a.err(format!("dict key must be {}, got {}", k.show(a), it.show(a)), span, module);
            }
            if **v != *vt && *vt != Ty::Err {
                a.err(
                    format!("dict holds {} values, got {}", v.show(a), vt.show(a)),
                    span,
                    module,
                );
            }
        }
        _ => a.err("only lists and dicts support index assignment".to_string(), span, module),
    }
}

fn infer_expr(e: &Expr, module: &str, a: &mut Analysis, ctx: &mut Ctx) -> Ty {
    let t = infer_expr_inner(e, module, a, ctx);
    a.ty.insert((module.to_string(), e.span.start), t.clone());
    if let Ty::List(_) | Ty::Dict(_, _) | Ty::Tuple(_, _) = &t {
        a.insts.insert(t.clone());
    }
    t
}

fn infer_expr_inner(e: &Expr, module: &str, a: &mut Analysis, ctx: &mut Ctx) -> Ty {
    match &e.kind {
        ExprKind::Int(_) => Ty::Int,
        ExprKind::Float(_) => Ty::Float,
        ExprKind::Bool(_) => Ty::Bool,
        ExprKind::Nil => Ty::Ptr,
        ExprKind::Str(_) => Ty::Str,
        ExprKind::FStr(parts) => {
            for p in parts {
                if let FStrPartAst::E(sub) = p {
                    let t = infer_expr(sub, module, a, ctx);
                    if !t.printable() {
                        a.err(
                            format!(
                                "cannot put {} into an f-string; printable values are int, float, bool, str",
                                t.show(a)
                            ),
                            sub.span,
                            module,
                        );
                    }
                }
            }
            Ty::Str
        }
        ExprKind::Ident(name) => {
            if let Some(t) = ctx.lookup(name) {
                return t;
            }
            // function reference
            let fm = format!("f_{}_{}", module, name);
            if a.fns.contains_key(&fm) {
                return Ty::Func(fm);
            }
            let cm = format!("c_{}_{}", module, name);
            if let Some(v) = a.consts.get(&cm) {
                return const_ty(v);
            }
            a.err(
                format!("unknown name '{}'; {}", name, {
                    let mut names: Vec<String> = ctx.scopes.iter().flat_map(|s| s.keys().cloned()).collect();
                    names.extend(a.fns.values().filter(|f| f.module == module).map(|f| f.name.clone()));
                    did_you_mean(name, &names.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                }),
                e.span,
                module,
            );
            Ty::Err
        }
        ExprKind::Unary(_, inner) => {
            let t = infer_expr(inner, module, a, ctx);
            if !t.is_numeric() && t != Ty::Err {
                a.err(format!("cannot negate {}", t.show(a)), e.span, module);
            }
            t
        }
        ExprKind::Binary(op, l, r) => {
            let lt = infer_expr(l, module, a, ctx);
            let rt = infer_expr(r, module, a, ctx);
            bin_type(*op, &lt, &rt, e.span, module, a)
        }
        ExprKind::And(l, r) | ExprKind::Or(l, r) => {
            let lt = infer_expr(l, module, a, ctx);
            let rt = infer_expr(r, module, a, ctx);
            if lt != Ty::Bool || rt != Ty::Bool {
                a.err("and/or need bool on both sides".to_string(), e.span, module);
            }
            Ty::Bool
        }
        ExprKind::Not(inner) => {
            let t = infer_expr(inner, module, a, ctx);
            if t != Ty::Bool && t != Ty::Err {
                a.err(format!("not needs bool, got {}", t.show(a)), e.span, module);
            }
            Ty::Bool
        }
        ExprKind::Compare(op, l, r) => {
            let lt = infer_expr(l, module, a, ctx);
            let rt = infer_expr(r, module, a, ctx);
            let comparable = (lt.is_numeric() && rt.is_numeric())
                || (lt == rt && matches!(lt, Ty::Str | Ty::Bool | Ty::Ptr | Ty::Err))
                || (lt == Ty::Ptr && rt == Ty::Ptr)
                || (matches!(op, CmpOp::Eq | CmpOp::Ne) && lt == rt && struct_comparable(&lt, a));
            if !comparable {
                a.err(
                    format!("cannot compare {} with {}", lt.show(a), rt.show(a)),
                    e.span,
                    module,
                );
            }
            Ty::Bool
        }
        ExprKind::Call(callee, args) => infer_call(callee, args, e, module, a, ctx),
        ExprKind::Method(recv, name, args) => infer_method(recv, name, args, e, module, a, ctx),
        ExprKind::Field(recv, name) => {
            // module-qualified constant: math.PI
            if let ExprKind::Ident(base) = &recv.kind {
                if !ctx.lookup(base).is_some() {
                    let cm = format!("c_{}_{}", base, name);
                    if a.consts.contains_key(&cm) {
                        let v = a.consts.get(&cm).unwrap().clone();
                        return const_ty(&v);
                    }
                }
            }
            let rt = infer_expr(recv, module, a, ctx);
            match rt {
                Ty::Struct(m) => {
                    let Some(si) = a.structs.get(&m) else { return Ty::Err };
                    match si.fields.iter().find(|(n, _)| n == name) {
                        Some((_, ft)) => ft.clone(),
                        None => {
                            let names: Vec<&str> = si.fields.iter().map(|(n, _)| n.as_str()).collect();
                            a.err(
                                format!(
                                    "struct '{}' has no field '{}'; {}",
                                    si.name,
                                    name,
                                    did_you_mean(name, &names)
                                ),
                                e.span,
                                module,
                            );
                            Ty::Err
                        }
                    }
                }
                other => {
                    a.err(
                        format!("cannot read field '{}' from {}", name, other.show(a)),
                        e.span,
                        module,
                    );
                    Ty::Err
                }
            }
        }
        ExprKind::Index(recv, idx) => {
            let rt = infer_expr(recv, module, a, ctx);
            let it = infer_expr(idx, module, a, ctx);
            if it != Ty::Int && it != Ty::Err {
                a.err(format!("index must be int, got {}", it.show(a)), e.span, module);
            }
            match rt {
                Ty::List(t) => *t,
                Ty::Dict(k, v) => {
                    let kt = it.clone();
                    if kt != *k && kt != Ty::Err {
                        a.err(
                            format!("dict keys are {}, got {}", k.show(a), kt.show(a)),
                            e.span,
                            module,
                        );
                    }
                    *v
                }
                Ty::Str => Ty::Str,
                other => {
                    a.err(
                        format!("cannot index {}", other.show(a)),
                        e.span,
                        module,
                    );
                    Ty::Err
                }
            }
        }
        ExprKind::Slice(recv, start, end) => {
            let rt = infer_expr(recv, module, a, ctx);
            if let Some(s) = start {
                let st = infer_expr(s, module, a, ctx);
                if st != Ty::Int && st != Ty::Err {
                    a.err("slice bounds must be int".to_string(), e.span, module);
                }
            }
            if let Some(x) = end {
                let et = infer_expr(x, module, a, ctx);
                if et != Ty::Int && et != Ty::Err {
                    a.err("slice bounds must be int".to_string(), e.span, module);
                }
            }
            match rt {
                Ty::Str => Ty::Str,
                Ty::List(t) => Ty::List(t),
                other => {
                    a.err(format!("cannot slice {}", other.show(a)), e.span, module);
                    Ty::Err
                }
            }
        }
        ExprKind::ListLit(items) => {
            let mut elem: Option<Ty> = None;
            for it in items {
                let t = infer_expr(it, module, a, ctx);
                match &elem {
                    None => elem = Some(t),
                    Some(prev) => {
                        if *prev != t && t != Ty::Err && *prev != Ty::Err {
                            a.err(
                                format!(
                                    "list elements disagree: {} then {}",
                                    prev.show(a),
                                    t.show(a)
                                ),
                                it.span,
                                module,
                            );
                        }
                    }
                }
            }
            match elem {
                Some(t) => Ty::List(Box::new(t)),
                None => {
                    a.err(
                        "cannot infer the element type of an empty list; annotate it: let xs: list[int] = []".to_string(),
                        e.span,
                        module,
                    );
                    Ty::List(Box::new(Ty::Err))
                }
            }
        }
        ExprKind::DictLit(pairs) => {
            let mut kt: Option<Ty> = None;
            let mut vt: Option<Ty> = None;
            for (k, v) in pairs {
                let k_t = infer_expr(k, module, a, ctx);
                let v_t = infer_expr(v, module, a, ctx);
                if let Some(p) = &kt {
                    if *p != k_t && k_t != Ty::Err {
                        a.err(
                            format!("dict keys disagree: {} then {}", p.show(a), k_t.show(a)),
                            k.span,
                            module,
                        );
                    }
                } else {
                    kt = Some(k_t);
                }
                if let Some(p) = &vt {
                    if *p != v_t && v_t != Ty::Err {
                        a.err(
                            format!("dict values disagree: {} then {}", p.show(a), v_t.show(a)),
                            v.span,
                            module,
                        );
                    }
                } else {
                    vt = Some(v_t);
                }
            }
            match (kt, vt) {
                (Some(k), Some(v)) => {
                    if k != Ty::Str && k != Ty::Int {
                        a.err(
                            "dict keys must be str or int in v0.1".to_string(),
                            e.span,
                            module,
                        );
                    }
                    Ty::Dict(Box::new(k), Box::new(v))
                }
                _ => {
                    a.err(
                        "cannot infer the types of an empty dict; annotate it: let d: dict[str, int] = {}".to_string(),
                        e.span,
                        module,
                    );
                    Ty::Dict(Box::new(Ty::Err), Box::new(Ty::Err))
                }
            }
        }
        ExprKind::TupleLit(x, y) => {
            let tx = infer_expr(x, module, a, ctx);
            let ty = infer_expr(y, module, a, ctx);
            Ty::Tuple(Box::new(tx), Box::new(ty))
        }
        ExprKind::Cast(t, inner) => {
            let it = infer_expr(inner, module, a, ctx);
            let target = conv_type(t, module, a);
            // int <-> ptr numeric casts, identity casts, and pointer
            // reinterpretation between ptr and RC types (str, list, dict).
            let ok = (target == Ty::Ptr && it == Ty::Int)
                || (target == Ty::Int && it == Ty::Ptr)
                || target == it
                || (target == Ty::Ptr && matches!(it, Ty::Str | Ty::List(_) | Ty::Dict(_, _)))
                || (it == Ty::Ptr && matches!(target, Ty::Str | Ty::List(_) | Ty::Dict(_, _)));
            if !ok {
                a.err(
                    "cast converts int <-> ptr, or reinterprets between ptr and str/list/dict".to_string(),
                    e.span,
                    module,
                );
            }
            target
        }
    }
}

fn const_ty(v: &ConstVal) -> Ty {
    match v {
        ConstVal::Int(_) => Ty::Int,
        ConstVal::Float(_) => Ty::Float,
        ConstVal::Bool(_) => Ty::Bool,
        ConstVal::Str(_) => Ty::Str,
    }
}

fn struct_comparable(t: &Ty, a: &Analysis) -> bool {
    if let Ty::Struct(m) = t {
        if let Some(si) = a.structs.get(m) {
            return si
                .fields
                .iter()
                .all(|(_, ft)| matches!(ft, Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Ptr));
        }
    }
    false
}

fn bin_type(op: BinOp, lt: &Ty, rt: &Ty, span: Span, module: &str, a: &mut Analysis) -> Ty {
    use BinOp::*;
    match op {
        Add => match (lt, rt) {
            (Ty::Int, Ty::Int) => Ty::Int,
            (Ty::Float, Ty::Float) | (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => Ty::Float,
            (Ty::Str, Ty::Str) => Ty::Str,
            (Ty::List(x), Ty::List(y)) if x == y => Ty::List(x.clone()),
            (Ty::Err, _) | (_, Ty::Err) => Ty::Err,
            (Ty::Str, other) => {
                a.err(
                    format!(
                        "cannot add {} to a str; use an f-string: f\"{{x}}\"",
                        other.show(a)
                    ),
                    span,
                    module,
                );
                Ty::Err
            }
            _ => {
                a.err(
                    format!("cannot add {} and {}", lt.show(a), rt.show(a)),
                    span,
                    module,
                );
                Ty::Err
            }
        },
        Sub | Mul | Mod => {
            if lt.is_numeric() && rt.is_numeric() {
                if lt == &Ty::Float || rt == &Ty::Float {
                    Ty::Float
                } else {
                    Ty::Int
                }
            } else if *lt == Ty::Err || *rt == Ty::Err {
                Ty::Err
            } else {
                a.err(
                    format!("{} needs numbers, got {} and {}", op_name(op), lt.show(a), rt.show(a)),
                    span,
                    module,
                );
                Ty::Err
            }
        }
        Div => {
            if lt.is_numeric() && rt.is_numeric() {
                if lt == &Ty::Float || rt == &Ty::Float {
                    Ty::Float
                } else {
                    Ty::Int
                }
            } else if *lt == Ty::Err || *rt == Ty::Err {
                Ty::Err
            } else {
                a.err("division needs numbers".to_string(), span, module);
                Ty::Err
            }
        }
        Pow => {
            if lt.is_numeric() && rt.is_numeric() {
                if lt == &Ty::Float || rt == &Ty::Float {
                    Ty::Float
                } else {
                    Ty::Int
                }
            } else if *lt == Ty::Err || *rt == Ty::Err {
                Ty::Err
            } else {
                a.err("** needs numbers".to_string(), span, module);
                Ty::Err
            }
        }
    }
}

fn op_name(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
    }
}

fn infer_call(
    callee: &Expr,
    args: &[Expr],
    call_expr: &Expr,
    module: &str,
    a: &mut Analysis,
    ctx: &mut Ctx,
) -> Ty {
    // Bare identifier call.
    if let ExprKind::Ident(name) = &callee.kind {
        // function value in a local?
        if let Some(Ty::Func(fm)) = ctx.lookup(name) {
            let fi = a.fns.get(&fm).cloned();
            if let Some(fi) = fi {
                check_args(&fi, args, call_expr, module, a, ctx);
                a.call.insert((module.to_string(), call_expr.span.start), CallTarget::Fn(fm));
                return fi.ret;
            }
        }
        // local struct-new? locals can't be structs. Struct constructor:
        let sm = format!("s_{}_{}", module, name);
        if a.structs.contains_key(&sm) {
            return struct_new(&sm, args, call_expr, module, a, ctx);
        }
        // extern declared in this module (so std wrappers can call the C symbol)
        let xm = format!("x_{}_{}", module, name);
        if let Some(fi) = a.fns.get(&xm).cloned() {
            check_args(&fi, args, call_expr, module, a, ctx);
            a.call.insert((module.to_string(), call_expr.span.start), CallTarget::Fn(xm));
            return fi.ret;
        }
        // function in this module
        let fm = format!("f_{}_{}", module, name);
        if let Some(fi) = a.fns.get(&fm).cloned() {
            if a.in_progress.contains(&fm) && matches!(fi.ret, Ty::Err) {
                a.err(
                    format!(
                        "fn '{}' calls itself, so it needs a declared return type, like: fn {}(...) -> int:",
                        name, name
                    ),
                    call_expr.span,
                    module,
                );
            }
            check_args(&fi, args, call_expr, module, a, ctx);
            a.call.insert((module.to_string(), call_expr.span.start), CallTarget::Fn(fm));
            return fi.ret;
        }
        // builtins
        match name.as_str() {
            "print" => {
                for arg in args {
                    let t = infer_expr(arg, module, a, ctx);
                    if !t.printable() {
                        a.err(
                            format!(
                                "print takes int, float, bool, str; got {}. Loop to print collections.",
                                t.show(a)
                            ),
                            arg.span,
                            module,
                        );
                    }
                }
                a.call.insert((module.to_string(), call_expr.span.start), CallTarget::BuiltinPrint);
                return Ty::Void;
            }
            "len" => {
                if args.len() != 1 {
                    a.err("len takes one argument".to_string(), call_expr.span, module);
                }
                let t = infer_expr(&args[0], module, a, ctx);
                match t {
                    Ty::Str | Ty::List(_) | Ty::Dict(_, _) => {}
                    other => {
                        a.err(
                            format!("len works on str, list, dict; not {}", other.show(a)),
                            call_expr.span,
                            module,
                        );
                    }
                }
                a.call.insert((module.to_string(), call_expr.span.start), CallTarget::BuiltinLen);
                return Ty::Int;
            }
            "float" => {
                if args.len() != 1 {
                    a.err("float takes one int".to_string(), call_expr.span, module);
                }
                let t = infer_expr(&args[0], module, a, ctx);
                if t != Ty::Int {
                    a.err(format!("float converts an int, got {}", t.show(a)), call_expr.span, module);
                }
                a.call.insert((module.to_string(), call_expr.span.start), CallTarget::BuiltinFloat);
                return Ty::Float;
            }
            "int" => {
                if args.len() != 1 {
                    a.err("int takes one float".to_string(), call_expr.span, module);
                }
                let t = infer_expr(&args[0], module, a, ctx);
                if t != Ty::Float {
                    a.err(format!("int converts a float, got {}", t.show(a)), call_expr.span, module);
                }
                a.call.insert((module.to_string(), call_expr.span.start), CallTarget::BuiltinInt);
                return Ty::Int;
            }
            "assert" => {
                if args.len() != 1 {
                    a.err("assert takes one bool".to_string(), call_expr.span, module);
                }
                let t = infer_expr(&args[0], module, a, ctx);
                if t != Ty::Bool {
                    a.err(format!("assert needs bool, got {}", t.show(a)), call_expr.span, module);
                }
                a.call.insert((module.to_string(), call_expr.span.start), CallTarget::BuiltinAssert);
                return Ty::Void;
            }
            _ => {}
        }
        a.err(
            format!("unknown function '{}'; {}", name, {
                let mut names: Vec<&str> = vec!["print", "len", "assert"];
                names.extend(
                    a.fns
                        .values()
                        .filter(|f| f.module == module && f.receiver.is_none())
                        .map(|f| f.name.as_str()),
                );
                did_you_mean(name, &names)
            }),
            callee.span,
            module,
        );
        for arg in args {
            infer_expr(arg, module, a, ctx);
        }
        return Ty::Err;
    }

    // Qualified call: module.fn(...), module.Constructor(...)
    if let ExprKind::Field(base, name) = &callee.kind {
        if let ExprKind::Ident(bn) = &base.kind {
            if ctx.lookup(bn).is_none() && a.imports.get(module).is_some_and(|v| v.iter().any(|m| m == bn)) {
                // struct constructor from a module
                let sm = format!("s_{}_{}", bn, name);
                if a.structs.contains_key(&sm) {
                    let si = a.structs.get(&sm).cloned();
                    if let Some(si) = si {
                        if !si.exported {
                            a.err(format!("struct '{}' is not exported", name), callee.span, module);
                        }
                    }
                    return struct_new(&sm, args, call_expr, module, a, ctx);
                }
                let fm = format!("f_{}_{}", bn, name);
                if let Some(fi) = a.fns.get(&fm).cloned() {
                    if !fi.exported {
                        a.err(format!("'{}' in {} is not exported", name, bn), callee.span, module);
                    }
                    check_args(&fi, args, call_expr, module, a, ctx);
                    a.call.insert((module.to_string(), call_expr.span.start), CallTarget::Fn(fm));
                    return fi.ret;
                }
                a.err(format!("module '{}' has no '{}'", bn, name), callee.span, module);
                for arg in args {
                    infer_expr(arg, module, a, ctx);
                }
                return Ty::Err;
            }
        }
    }

    a.err("this expression cannot be called".to_string(), callee.span, module);
    for arg in args {
        infer_expr(arg, module, a, ctx);
    }
    Ty::Err
}

fn struct_new(sm: &str, args: &[Expr], call_expr: &Expr, module: &str, a: &mut Analysis, ctx: &mut Ctx) -> Ty {
    let si = a.structs.get(sm).cloned();
    let Some(si) = si else {
        return Ty::Err;
    };
    if args.len() != si.fields.len() {
        a.err(
            format!(
                "struct {} takes {} arguments ({}), got {}",
                si.name,
                si.fields.len(),
                si.fields.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", "),
                args.len()
            ),
            call_expr.span,
            module,
        );
    }
    for (arg, (fname, ft)) in args.iter().zip(si.fields.iter()) {
        let t = infer_expr(arg, module, a, ctx);
        if t != *ft && t != Ty::Err {
            a.err(
                format!("field '{}' wants {}, got {}", fname, ft.show(a), t.show(a)),
                arg.span,
                module,
            );
        }
    }
    a.call.insert((module.to_string(), call_expr.span.start), CallTarget::StructNew(sm.to_string()));
    Ty::Struct(sm.to_string())
}

fn check_args(fi: &FnInfo, args: &[Expr], call_expr: &Expr, module: &str, a: &mut Analysis, ctx: &mut Ctx) {
    if args.len() != fi.params.len() {
        a.err(
            format!(
                "'{}' takes {} argument{}, got {}",
                fi.name,
                fi.params.len(),
                if fi.params.len() == 1 { "" } else { "s" },
                args.len()
            ),
            call_expr.span,
            module,
        );
    }
    for (arg, p) in args.iter().zip(fi.params.iter()) {
        let t = infer_expr(arg, module, a, ctx);
        if t != p.ty && t != Ty::Err && p.ty != Ty::Err {
            a.err(
                format!("parameter '{}' wants {}, got {}", p.name, p.ty.show(a), t.show(a)),
                arg.span,
                module,
            );
        }
    }
}

fn infer_method(
    recv: &Expr,
    name: &str,
    args: &[Expr],
    call_expr: &Expr,
    module: &str,
    a: &mut Analysis,
    ctx: &mut Ctx,
) -> Ty {
    // A qualified module call parses as a method: math.sqrt(x).
    if let ExprKind::Ident(bn) = &recv.kind {
        if ctx.lookup(bn).is_none() && a.imports.get(module).is_some_and(|v| v.iter().any(|m| m == bn)) {
            let fm = format!("f_{}_{}", bn, name);
            if let Some(fi) = a.fns.get(&fm).cloned() {
                if !fi.exported {
                    a.err(format!("'{}' in {} is not exported", name, bn), call_expr.span, module);
                }
                check_args(&fi, args, call_expr, module, a, ctx);
                a.call.insert((module.to_string(), call_expr.span.start), CallTarget::Fn(fm));
                return fi.ret;
            }
            let sm = format!("s_{}_{}", bn, name);
            if a.structs.contains_key(&sm) {
                return struct_new(&sm, args, call_expr, module, a, ctx);
            }
            a.err(format!("module '{}' has no '{}'", bn, name), recv.span, module);
            for arg in args {
                infer_expr(arg, module, a, ctx);
            }
            return Ty::Err;
        }
    }

    let rt = infer_expr(recv, module, a, ctx);

    // Special bridging methods.
    match (name, &rt) {
        ("to_cstr", Ty::Str) => {
            a.call.insert((module.to_string(), call_expr.span.start), CallTarget::ToCStr);
            return Ty::Ptr;
        }
        ("to_str", Ty::Int | Ty::Float | Ty::Bool) => {
            a.call.insert((module.to_string(), call_expr.span.start), CallTarget::ToStr);
            return Ty::Str;
        }
        _ => {}
    }

    let ret = match &rt {
        Ty::List(elem) => {
            let op = match name {
                "push" => Some(ListOp::Push),
                "pop" => Some(ListOp::Pop),
                "insert" => Some(ListOp::Insert),
                "remove_at" => Some(ListOp::RemoveAt),
                "sort" => Some(ListOp::Sort),
                "reverse" => Some(ListOp::Reverse),
                "contains" => Some(ListOp::Contains),
                "index_of" => Some(ListOp::IndexOf),
                _ => None,
            };
            match op {
                Some(op) => {
                    let expected: Vec<Ty> = match op {
                        ListOp::Push | ListOp::Contains | ListOp::IndexOf => {
                            vec![elem.as_ref().clone()]
                        }
                        ListOp::Insert => vec![Ty::Int, elem.as_ref().clone()],
                        ListOp::RemoveAt => vec![Ty::Int],
                        _ => vec![],
                    };
                    let arity_ok = match op {
                        ListOp::Push | ListOp::Contains | ListOp::IndexOf => args.len() == 1,
                        ListOp::Insert => args.len() == 2,
                        ListOp::RemoveAt => args.len() == 1,
                        _ => args.is_empty(),
                    };
                    if !arity_ok {
                        a.err(
                            format!("list.{} takes {} argument(s)", name, expected_len(&op)),
                            call_expr.span,
                            module,
                        );
                    }
                    for (arg, exp) in args.iter().zip(expected.iter()) {
                        let t = infer_expr(arg, module, a, ctx);
                        if t != *exp && t != Ty::Err {
                            a.err(
                                format!("list.{} wants {}, got {}", name, exp.show(a), t.show(a)),
                                arg.span,
                                module,
                            );
                        }
                    }
                    a.call.insert(
                        (module.to_string(), call_expr.span.start),
                        CallTarget::ListMethod(op),
                    );
                    match op {
                        ListOp::Pop => elem.as_ref().clone(),
                        ListOp::Contains => Ty::Bool,
                        ListOp::IndexOf => Ty::Int,
                        _ => Ty::Void,
                    }
                }
                None => {
                    a.err(
                        format!(
                            "list[{}] has no method '{}'; methods: push, pop, insert, remove_at, sort, reverse, contains, index_of",
                            elem.show(a),
                            name
                        ),
                        call_expr.span,
                        module,
                    );
                    for arg in args {
                        infer_expr(arg, module, a, ctx);
                    }
                    Ty::Err
                }
            }
        }
        Ty::Dict(k, v) => {
            let op = match name {
                "get" => Some(DictOp::Get),
                "set" => Some(DictOp::Set),
                "keys" => Some(DictOp::Keys),
                "values" => Some(DictOp::Values),
                "items" => Some(DictOp::Items),
                "contains" => Some(DictOp::Contains),
                "remove" => Some(DictOp::Remove),
                _ => None,
            };
            match op {
                Some(op) => {
                    let expected: Vec<Ty> = match op {
                        DictOp::Get => vec![k.as_ref().clone(), v.as_ref().clone()],
                        DictOp::Set => vec![k.as_ref().clone(), v.as_ref().clone()],
                        DictOp::Contains | DictOp::Remove => vec![k.as_ref().clone()],
                        _ => vec![],
                    };
                    let arity_ok = match op {
                        DictOp::Get | DictOp::Set => args.len() == 2,
                        DictOp::Contains | DictOp::Remove => args.len() == 1,
                        _ => args.is_empty(),
                    };
                    if !arity_ok {
                        a.err(
                            format!("dict.{} takes {} argument(s)", name, expected_len_dict(&op)),
                            call_expr.span,
                            module,
                        );
                    }
                    for (arg, exp) in args.iter().zip(expected.iter()) {
                        let t = infer_expr(arg, module, a, ctx);
                        if t != *exp && t != Ty::Err {
                            a.err(
                                format!("dict.{} wants {}, got {}", name, exp.show(a), t.show(a)),
                                arg.span,
                                module,
                            );
                        }
                    }
                    a.call.insert(
                        (module.to_string(), call_expr.span.start),
                        CallTarget::DictMethod(op),
                    );
                    match op {
                        DictOp::Get => v.as_ref().clone(),
                        DictOp::Keys => Ty::List(k.clone()),
                        DictOp::Values => Ty::List(v.clone()),
                        DictOp::Items => Ty::List(Box::new(Ty::Tuple(k.clone(), v.clone()))),
                        DictOp::Contains => Ty::Bool,
                        _ => Ty::Void,
                    }
                }
                None => {
                    a.err(
                        format!(
                            "dict has no method '{}'; methods: get, set, keys, values, items, contains, remove",
                            name
                        ),
                        call_expr.span,
                        module,
                    );
                    for arg in args {
                        infer_expr(arg, module, a, ctx);
                    }
                    Ty::Err
                }
            }
        }
        Ty::Struct(sm) => {
            let si = a.structs.get(sm).cloned();
            match si {
                Some(si) => match si.methods.get(name) {
                    Some(fm) => {
                        let fi = a.fns.get(fm).cloned();
                        if let Some(fi) = fi {
                            // receiver is the implicit first argument
                            if args.len() + 1 != fi.params.len() {
                                a.err(
                                    format!(
                                        "method '{}' takes {} argument(s) plus self",
                                        name,
                                        fi.params.len() - 1
                                    ),
                                    call_expr.span,
                                    module,
                                );
                            }
                            for (arg, p) in args.iter().zip(fi.params.iter().skip(1)) {
                                let t = infer_expr(arg, module, a, ctx);
                                if t != p.ty && t != Ty::Err {
                                    a.err(
                                        format!("parameter '{}' wants {}, got {}", p.name, p.ty.show(a), t.show(a)),
                                        arg.span,
                                        module,
                                    );
                                }
                            }
                            a.call.insert(
                                (module.to_string(), call_expr.span.start),
                                CallTarget::FnSelf(fm.clone()),
                            );
                            fi.ret
                        } else {
                            Ty::Err
                        }
                    }
                    None => {
                        let mut names: Vec<&str> =
                            si.fields.iter().map(|(n, _)| n.as_str()).collect();
                        names.extend(si.methods.keys().map(|s| s.as_str()));
                        a.err(
                            format!(
                                "struct '{}' has no method '{}'; {}",
                                si.name,
                                name,
                                did_you_mean(name, &names)
                            ),
                            call_expr.span,
                            module,
                        );
                        for arg in args {
                            infer_expr(arg, module, a, ctx);
                        }
                        Ty::Err
                    }
                },
                None => Ty::Err,
            }
        }
        // str methods: the std str module, receiver as first argument.
        Ty::Str => {
            let fm = format!("f_str_{}", name);
            let fi = a.fns.get(&fm).cloned();
            match fi {
                Some(fi) => {
                    if args.len() + 1 != fi.params.len() {
                        a.err(
                            format!(
                                "str.{} takes {} argument(s)",
                                name,
                                fi.params.len().saturating_sub(1)
                            ),
                            call_expr.span,
                            module,
                        );
                    }
                    for (arg, p) in args.iter().zip(fi.params.iter().skip(1)) {
                        let t = infer_expr(arg, module, a, ctx);
                        if t != p.ty && t != Ty::Err {
                            a.err(
                                format!("parameter '{}' wants {}, got {}", p.name, p.ty.show(a), t.show(a)),
                                arg.span,
                                module,
                            );
                        }
                    }
                    a.call.insert(
                        (module.to_string(), call_expr.span.start),
                        CallTarget::FnSelf(fm.clone()),
                    );
                    fi.ret
                }
                None => {
                    a.err(
                        format!(
                            "str has no method '{}'; str module: upper, lower, strip, split, join, replace, contains, find, starts_with, ends_with, to_int, to_float, to_cstr",
                            name
                        ),
                        call_expr.span,
                        module,
                    );
                    for arg in args {
                        infer_expr(arg, module, a, ctx);
                    }
                    Ty::Err
                }
            }
        }
        other => {
            a.err(
                format!("{} has no methods", other.show(a)),
                call_expr.span,
                module,
            );
            for arg in args {
                infer_expr(arg, module, a, ctx);
            }
            Ty::Err
        }
    };
    ret
}

fn expected_len(op: &ListOp) -> usize {
    match op {
        ListOp::Push | ListOp::Contains | ListOp::IndexOf | ListOp::RemoveAt => 1,
        ListOp::Insert => 2,
        _ => 0,
    }
}

fn expected_len_dict(op: &DictOp) -> usize {
    match op {
        DictOp::Get | DictOp::Set => 2,
        DictOp::Contains | DictOp::Remove => 1,
        _ => 0,
    }
}

/// Levenshtein-based "did you mean" helper.
pub fn did_you_mean(name: &str, candidates: &[&str]) -> String {
    let mut best: Option<(usize, &str)> = None;
    for c in candidates {
        let d = levenshtein(name, c);
        let limit = (name.len() / 3).max(1) + 1;
        if d <= limit {
            match best {
                Some((bd, _)) if bd <= d => {}
                _ => best = Some((d, c)),
            }
        }
    }
    match best {
        Some((_, c)) => format!("did you mean '{}'?", c),
        None => String::new(),
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
