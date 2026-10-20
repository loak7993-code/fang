//! C code generation.
//!
//! Emits a single C translation unit per program: pooled string literals,
//! struct definitions, collection stamps, function prototypes and bodies,
//! and the entry wrapper. `#line` directives map C diagnostics back to
//! Fang source lines.

use crate::ast::*;
use crate::sema::{Analysis, CallTarget, ConstVal, DictOp, ListOp, Ty};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;

#[derive(Debug, Clone)]
pub struct SourceUnit {
    pub name: String,
    pub path: String,
    pub module: Module,
}

pub struct GenOpts {
    pub freestanding: bool,
}

/// Generate the full C source for a program.
/// One cached compilation unit: a module's C plus its header.
pub struct ModUnit {
    pub module: String,
    pub c: String,
    pub h: String,
}

/// A program split into per-module units: every non-entry module is
/// content-stable across programs that import it, so its object file
/// can be cached. Only the entry TU changes per program.
pub struct Program {
    pub mods: Vec<ModUnit>,
    pub entry_c: String,
    /// Init order for module globals: imports first, entry last.
    pub ginit_order: Vec<String>,
}

pub fn generate_modular(
    sources: &[SourceUnit],
    a: &crate::sema::Analysis,
    entry: &str,
    opts: &GenOpts,
) -> Result<Program, String> {
    let mut g = Gen {
        sources,
        reachable: HashSet::new(),
        a,
        opts,
        out: String::new(),
        pool: HashMap::new(),
        pool_order: Vec::new(),
        used_headers: BTreeSet::new(),
        used_eq_helpers: HashSet::new(),
        extra_decls: std::cell::RefCell::new(String::new()),
        extra_fns: std::cell::RefCell::new(String::new()),
        stamped_sigs: std::cell::RefCell::new(HashSet::new()),
        fns_external: false,
    };
    g.run_modular(entry)
}

pub fn generate(
    sources: &[SourceUnit],
    a: &Analysis,
    entry: &str,
    opts: &GenOpts,
) -> Result<String, String> {
    let mut g = Gen {
        sources,
        reachable: HashSet::new(),
        a,
        opts,
        out: String::new(),
        pool: HashMap::new(),
        pool_order: Vec::new(),
        used_headers: BTreeSet::new(),
        used_eq_helpers: HashSet::new(),
        extra_decls: std::cell::RefCell::new(String::new()),
        extra_fns: std::cell::RefCell::new(String::new()),
        stamped_sigs: std::cell::RefCell::new(HashSet::new()),
        fns_external: false,
    };
    g.run(entry)
}

struct Gen<'a> {
    sources: &'a [SourceUnit],
    /// Modules reachable from the entry through imports. Only these
    /// are emitted; the rest of the embedded stdlib is dropped.
    reachable: HashSet<String>,
    a: &'a Analysis,
    opts: &'a GenOpts,
    out: String,
    pool: HashMap<String, String>,
    pool_order: Vec<String>,
    used_headers: BTreeSet<String>,
    used_eq_helpers: HashSet<String>,
    /// Trampolines, env structs, adapters: emitted after fn bodies.
    extra_decls: std::cell::RefCell<String>,
    /// Lambda body functions: emitted after fn bodies.
    extra_fns: std::cell::RefCell<String>,
    /// Signature keys already stamped.
    stamped_sigs: std::cell::RefCell<HashSet<String>>,
    /// Module TUs get external fn linkage (header prototypes match).
    pub(crate) fns_external: bool,
}

impl<'a> Gen<'a> {
    /// Dependency edges of a module: imports plus method-sugar call
    /// edges (s.find(..) reaches the str module without an import).
    fn module_deps(&self, m: &str) -> Vec<String> {
        let mut deps: Vec<String> = Vec::new();
        if let Some(ims) = self.a.imports.get(m) {
            for im in ims.clone() {
                if !deps.contains(&im) {
                    deps.push(im);
                }
            }
        }
        for ((cm, _), target) in &self.a.call {
            if cm != m {
                continue;
            }
            let callee_module = match target {
                crate::sema::CallTarget::Fn(mangled)
                | crate::sema::CallTarget::FnSelf(mangled) => {
                    self.a.fns.get(mangled).map(|fi| fi.module.clone())
                }
                crate::sema::CallTarget::StructNew(mangled) => self
                    .a
                    .structs
                    .get(mangled)
                    .map(|si| si.module.clone()),
                _ => None,
            };
            if let Some(cm2) = callee_module {
                if cm2 != m && !deps.contains(&cm2) {
                    deps.push(cm2);
                }
            }
        }
        deps
    }

    /// Topological order over import+call edges, deps first, entry last.
    fn topo_order(&self, entry: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        fn visit(
            m: &str,
            seen: &mut HashSet<String>,
            out: &mut Vec<String>,
            deps_of: &dyn Fn(&str) -> Vec<String>,
        ) {
            if !seen.insert(m.to_string()) {
                return;
            }
            for d in deps_of(m) {
                visit(&d, seen, out, deps_of);
            }
            out.push(m.to_string());
        }
        let g = self;
        let deps_of = move |m: &str| g.module_deps(m);
        visit(entry, &mut seen, &mut out, &deps_of);
        out
    }

    fn module_insts(&self, scope: &str) -> BTreeSet<Ty> {
        let mut out: BTreeSet<Ty> = BTreeSet::new();
        let mut work: Vec<Ty> = Vec::new();
        for fi in self.a.fns.values() {
            if fi.module != scope {
                continue;
            }
            for p in &fi.params {
                work.push(p.ty.clone());
            }
            work.push(fi.ret.clone());
        }
        for si in self.a.structs.values() {
            if si.module != scope {
                continue;
            }
            for (_, t) in &si.fields {
                work.push(t.clone());
            }
        }
        work.extend(
            self.a
                .ty
                .iter()
                .filter(|((m, _), _)| m == scope)
                .map(|(_, t)| t.clone()),
        );
        work.extend(
            self.a
                .for_binders
                .iter()
                .filter(|((m, _), _)| m == scope)
                .flat_map(|(_, v)| v.iter().cloned()),
        );
        work.extend(
            self.a
                .catch_ty
                .iter()
                .filter(|((m, _), _)| m == scope)
                .map(|(_, t)| t.clone()),
        );
        while let Some(t) = work.pop() {
            match &t {
                Ty::List(e) => {
                    if out.insert(t.clone()) {
                        work.push((**e).clone());
                    }
                }
                Ty::Dict(k, v) => {
                    if out.insert(t.clone()) {
                        let kv = Ty::Tuple(Box::new((**k).clone()), Box::new((**v).clone()));
                        out.insert(Ty::List(Box::new(kv.clone())));
                        out.insert(kv);
                        out.insert(Ty::List(Box::new((**k).clone())));
                        out.insert(Ty::List(Box::new((**v).clone())));
                        work.push((**k).clone());
                        work.push((**v).clone());
                    }
                }
                Ty::Tuple(a, b) if out.insert(t.clone()) => {
                    work.push((**a).clone());
                    work.push((**b).clone());
                }
                _ => {}
            }
        }
        out
    }

    /// Which modules' headers does this module need (struct-by-value use)?
    fn header_deps(&self, scope: &str) -> Vec<String> {
        fn scan(
            t: &Ty,
            scope: &str,
            structs: &HashMap<String, crate::sema::StructInfo>,
            deps: &mut Vec<String>,
        ) {
            match t {
                Ty::Struct(m) => {
                    if let Some(si) = structs.get(m) {
                        if si.module != scope && !deps.contains(&si.module) {
                            deps.push(si.module.clone());
                        }
                    }
                }
                Ty::List(e) => scan(e, scope, structs, deps),
                Ty::Dict(k, v) => {
                    scan(k, scope, structs, deps);
                    scan(v, scope, structs, deps);
                }
                Ty::Tuple(a, b) => {
                    scan(a, scope, structs, deps);
                    scan(b, scope, structs, deps);
                }
                _ => {}
            }
        }
        let mut deps: Vec<String> = Vec::new();
        for fi in self.a.fns.values() {
            if fi.module != scope {
                continue;
            }
            for p in &fi.params {
                scan(&p.ty, scope, &self.a.structs, &mut deps);
            }
            scan(&fi.ret, scope, &self.a.structs, &mut deps);
        }
        for si in self.a.structs.values() {
            if si.module != scope {
                continue;
            }
            for (_, t) in &si.fields {
                scan(t, scope, &self.a.structs, &mut deps);
            }
        }
        deps
    }

    fn emit_forward_typedefs(&mut self, insts: &BTreeSet<Ty>) {
        for t in insts {
            match t {
                Ty::List(e) => {
                    let es = self.sfx(e);
                    writeln!(self.out, "typedef struct fang_list_{0} fang_list_{0};", es).unwrap();
                }
                Ty::Dict(k, v) => {
                    let ks = self.sfx(k);
                    let vs = self.sfx(v);
                    writeln!(self.out, "typedef struct fang_dict_{0}_{1} fang_dict_{0}_{1};", ks, vs).unwrap();
                    writeln!(self.out, "typedef struct fang_dslot_{0}_{1} fang_dslot_{0}_{1};", ks, vs).unwrap();
                }
                Ty::Tuple(a, b) => {
                    let asx = self.sfx(a);
                    let bsx = self.sfx(b);
                    writeln!(self.out, "typedef struct fang_tuple_{0}_{1} fang_tuple_{0}_{1};", asx, bsx).unwrap();
                }
                _ => {}
            }
        }
    }

    fn emit_stamps(&mut self, insts: &BTreeSet<Ty>) -> Result<(), String> {
        for t in insts {
            if let Ty::Tuple(x, y) = t {
                writeln!(
                    self.out,
                    "FANG_TUPLE_DEFINE({}, {}, {}, {})",
                    self.sfx(x),
                    self.sfx(y),
                    self.ctype(x),
                    self.ctype(y)
                )
                .map_err(|e| e.to_string())?;
            }
        }
        for t in insts {
            if let Ty::List(e) = t {
                self.emit_list_stamp(e);
            }
        }
        for t in insts {
            if let Ty::Dict(k, v) = t {
                self.emit_dict_stamp(k, v);
            }
        }
        Ok(())
    }

    fn struct_header_part(&mut self, mangled: &str, h: &mut String) {
        let Some(si) = self.a.structs.get(mangled) else { return; };
        let sname = format!("fang_{}", mangled);
        writeln!(h, "typedef struct {} {{", sname).unwrap();
        for (fname, ft) in &si.fields {
            let cty = match ft {
                Ty::Str => {
                    if self.opts.freestanding {
                        "void*".to_string()
                    } else {
                        "fang_str*".into()
                    }
                }
                other => self.ctype(other),
            };
            writeln!(h, "    {} {};", cty, fname).unwrap();
        }
        writeln!(h, "}} {};", sname).unwrap();
        let params: Vec<String> = si
            .fields
            .iter()
            .enumerate()
            .map(|(i, (_, t))| format!("{} a{}", self.ctype(t), i))
            .collect();
        writeln!(
            h,
            "{} {}_new({});",
            sname,
            sname,
            params.join(", ")
        )
        .unwrap();
        let comparable = si
            .fields
            .iter()
            .all(|(_, t)| matches!(t, Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Ptr));
        if comparable {
            writeln!(h, "bool {}_eq({}, {} b);", sname, sname, sname).unwrap();
        }
    }

    fn struct_body_part(&mut self, mangled: &str) {
        let Some(si) = self.a.structs.get(mangled) else { return; };
        let sname = format!("fang_{}", mangled);
        let params: Vec<String> = si
            .fields
            .iter()
            .enumerate()
            .map(|(i, (_, t))| format!("{} a{}", self.ctype(t), i))
            .collect();
        let assigns: Vec<String> = si
            .fields
            .iter()
            .enumerate()
            .map(|(i, (n, _))| format!("s.{} = a{};", n, i))
            .collect();
        writeln!(
            self.out,
            "{} {}_new({}) {{\n    {} s;\n    {}\n    return s;\n}}",
            sname,
            sname,
            params.join(", "),
            sname,
            assigns.join("\n    ")
        )
        .unwrap();
        let comparable = si
            .fields
            .iter()
            .all(|(_, t)| matches!(t, Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Ptr));
        if comparable {
            let conds: Vec<String> = si
                .fields
                .iter()
                .map(|(n, t)| match t {
                    Ty::Str => format!("fang_str_eq(a.{}, b.{})", n, n),
                    _ => format!("a.{} == b.{}", n, n),
                })
                .collect();
            let cond = if conds.is_empty() {
                "true".to_string()
            } else {
                conds.join(" && ")
            };
            writeln!(
                self.out,
                "bool {}_eq({} a, {} b) {{\n    return {};\n}}",
                sname, sname, sname, cond
            )
            .unwrap();
        }
    }

    fn run_modular(&mut self, entry: &str) -> Result<Program, String> {
        // Reachability, same as the single-TU path.
        let mut reach = HashSet::new();
        let mut queue = vec![entry.to_string()];
        while let Some(m) = queue.pop() {
            if !reach.insert(m.clone()) {
                continue;
            }
            if let Some(imports) = self.a.imports.get(&m) {
                for im in imports.clone() {
                    queue.push(im);
                }
            }
            for ((cm, _), target) in &self.a.call {
                if cm != &m {
                    continue;
                }
                let callee_module = match target {
                    crate::sema::CallTarget::Fn(mangled)
                    | crate::sema::CallTarget::FnSelf(mangled) => {
                        self.a.fns.get(mangled).map(|fi| fi.module.clone())
                    }
                    crate::sema::CallTarget::StructNew(mangled) => self
                        .a
                        .structs
                        .get(mangled)
                        .map(|si| si.module.clone()),
                    _ => None,
                };
                if let Some(cm2) = callee_module {
                    if !reach.contains(&cm2) {
                        queue.push(cm2);
                    }
                }
            }
        }
        self.reachable = reach;

        let order = self.topo_order(entry);
        let mut mods: Vec<ModUnit> = Vec::new();

        for mname in &order {
            if mname == entry {
                continue;
            }
            let u = match self.sources.iter().find(|u| &u.name == mname) {
                Some(u) => u,
                None => continue,
            };
            // Header: includes, struct defs, prototypes, extern globals.
            let mut h = String::new();
            writeln!(h, "/* fang module header: {} */", mname).unwrap();
            writeln!(h, "#pragma once").unwrap();
            writeln!(h, "#include \"fang_rt.h\"").unwrap();
            for dep in self.header_deps(mname) {
                if dep != *mname && order.contains(&dep) && dep != *entry {
                    writeln!(h, "#include \"fang_mod_{dep}.h\"", dep = dep).unwrap();
                }
            }
            writeln!(h).unwrap();
            // extern headers used by this module's extern fns
            let mut mheaders: BTreeSet<String> = BTreeSet::new();
            for item in &u.module.items {
                if let Item::ExternFn { header, .. } = item {
                    let ht = header.trim();
                    if !ht.is_empty()
                        && ht.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
                    {
                        mheaders.insert(ht.to_string());
                    }
                }
            }
            for hh in &mheaders {
                writeln!(h, "#include <{}>", hh).unwrap();
            }
            let h_insts = self.module_insts(mname);
            {
                let mut hh = h.clone();
                let saved = std::mem::take(&mut self.out);
                self.emit_forward_typedefs(&h_insts);
                hh.push_str(&self.out);
                self.out = saved;
                h = hh;
            }
            writeln!(h).unwrap();
            // struct defs owned by this module
            let mut h2 = h.clone();
            let mut owned: Vec<String> = self
                .a
                .structs
                .values()
                .filter(|si| &si.module == mname)
                .map(|si| si.mangled.clone())
                .collect();
            owned.sort();
            for mangled in &owned {
                self.struct_header_part(mangled, &mut h2);
            }
            h = h2;
            writeln!(h).unwrap();
            // prototypes + extern globals + ginit declaration
            let mut protos = String::new();
            let mut mod_fns: Vec<&crate::sema::FnInfo> = self
                .a
                .fns
                .values()
                .filter(|fi| fi.module == *mname && !fi.is_extern)
                .collect();
            mod_fns.sort_by(|x, y| x.mangled.cmp(&y.mangled));
            for fi in mod_fns {
                let params = self.fn_params(fi);
                let ret = self.ctype_ret(&fi.ret);
                writeln!(protos, "{} {}({});", ret, fi.mangled, params).unwrap();
            }
            writeln!(protos, "void fang_ginit_{}(void);", mname).unwrap();
            h.push_str(&protos);
            writeln!(h).unwrap();
            for item in &u.module.items {
                if let Item::Const { name, .. } = item {
                    let gm = format!("g_{}_{}", mname, name);
                    if let Some(ty) = self.a.globals.get(&gm) {
                        writeln!(h, "extern {} {};", self.ctype(ty), gm).unwrap();
                    }
                }
            }

            // C body for the module.
            let mut c = String::new();
            writeln!(c, "/* fang module: {} */", mname).unwrap();
            writeln!(c, "#include \"fang_mod_{m}.h\"", m = mname).unwrap();
            // Callee modules: imports plus call edges (method sugar
            // reaches str without an import). Topo order.
            {
                let mut callees: Vec<String> = Vec::new();
                if let Some(ims) = self.a.imports.get(mname) {
                    for im in ims.clone() {
                        if !callees.contains(&im) {
                            callees.push(im);
                        }
                    }
                }
                for ((cm, _), target) in &self.a.call {
                    if cm != mname {
                        continue;
                    }
                    let callee_module = match target {
                        crate::sema::CallTarget::Fn(mangled)
                        | crate::sema::CallTarget::FnSelf(mangled) => {
                            self.a.fns.get(mangled).map(|fi| fi.module.clone())
                        }
                        crate::sema::CallTarget::StructNew(mangled) => self
                            .a
                            .structs
                            .get(mangled)
                            .map(|si| si.module.clone()),
                        _ => None,
                    };
                    if let Some(cm2) = callee_module {
                        if cm2 != *mname && !callees.contains(&cm2) {
                            callees.push(cm2);
                        }
                    }
                }
                for dep in &order {
                    if callees.contains(dep) && dep != mname && dep != entry {
                        writeln!(c, "#include \"fang_mod_{m}.h\"", m = dep).unwrap();
                    }
                }
            }
            writeln!(c).unwrap();
            let insts = self.module_insts(mname);
            {
                let saved = std::mem::take(&mut self.out);
                self.emit_forward_typedefs(&insts);
                c.push_str(&self.out);
                self.out = saved;
            }
            writeln!(c).unwrap();
            // module-owned struct function bodies
            let saved = std::mem::take(&mut self.out);
            for mangled in &owned {
                self.struct_body_part(mangled);
            }
            c.push_str(&self.out);
            self.out = saved;
            writeln!(c).unwrap();
            // stamps (static: each TU carries its own copies; the
            // linker strips unreferenced sections)
            let saved = std::mem::take(&mut self.out);
            self.emit_stamps(&insts)?;
            c.push_str(&self.out);
            self.out = saved;
            writeln!(c).unwrap();
            // module string pool
            let mut pool: HashMap<String, String> = HashMap::new();
            let mut pool_order: Vec<String> = Vec::new();
            collect_strs(&u.module, &mut pool, &mut pool_order);
            for (i, _t) in pool_order.iter().enumerate() {
                writeln!(c, "static fang_str* L{};", i).unwrap();
            }
            writeln!(
                c,
                "static void fang_init_strings_{m}(void) {{",
                m = mname
            )
            .unwrap();
            for (i, text) in pool_order.iter().enumerate() {
                writeln!(
                    c,
                    "    L{i} = fang_str_new(\"{text}\", {len}); L{i}->rc = FANG_RC_STATIC;",
                    i = i,
                    text = c_escape(text),
                    len = text.chars().count()
                )
                .unwrap();
            }
            writeln!(c, "}}").unwrap();
            // global definitions
            let mut has_globals = false;
            for item in &u.module.items {
                if let Item::Const { name, .. } = item {
                    let gm = format!("g_{}_{}", mname, name);
                    if let Some(ty) = self.a.globals.get(&gm) {
                        writeln!(c, "{} {};", self.ctype(ty), gm).unwrap();
                        has_globals = true;
                    }
                }
            }
            if has_globals {
                writeln!(c).unwrap();
            }
            // bodies
            let saved = std::mem::take(&mut self.out);
            *self.extra_decls.borrow_mut() = String::new();
            *self.extra_fns.borrow_mut() = String::new();
            *self.stamped_sigs.borrow_mut() = HashSet::new();
            self.pool = pool.clone();
            self.pool_order = pool_order.clone();
            self.fns_external = true;
            for item in &u.module.items {
                if let Item::Fn(f) = item {
                    let mangled = match &f.receiver {
                        Some(r) => format!("f_{}_{}_{}", u.name, r, f.name),
                        None => format!("f_{}_{}", u.name, f.name),
                    };
                    let fi = self.a.fns[&mangled].clone();
                    self.emit_fn(u, f, &fi);
                }
            }
            self.fns_external = false;
            let extra_d = self.extra_decls.borrow().clone();
            let extra_f = self.extra_fns.borrow().clone();
            let extras = format!("{}{}", extra_d, extra_f);
            let bodies = std::mem::replace(&mut self.out, saved);
            c.push_str(&extras);
            writeln!(c).unwrap();
            c.push_str(&bodies);
            writeln!(c).unwrap();
            // Module init: strings, then one init function per global
            // (separate functions keep temp names scoped).
            let mut ginit_calls = String::new();
            for item in &u.module.items {
                if let Item::Const { name, value, .. } = item {
                    let gm = format!("g_{}_{}", mname, name);
                    if self.a.globals.contains_key(&gm) {
                        let mut body = String::new();
                        let path = u.path.clone();
                        let mut fg = FnGen {
                            module: mname.clone(),
                            path,
                            a: self.a,
                            captures: HashMap::new(),
                            buf: String::new(),
                            indent: 1,
                            scopes: vec![HashMap::new()],
                            frame: String::new(),
                            field_n: 0,
                            temp_n: 0,
                            locals: Vec::new(),
                            has_temps: Vec::new(),
                            ret: Ty::Void,
                        };
                        let ty = self.a.globals.get(&gm).cloned().unwrap_or(Ty::Err);
                        let (code, _t, fresh) = fg.gen_expr(value, self);
                        let assign = if self.is_rc(&ty) {
                            let incref = if fresh { String::new() } else {
                                format!("fang_incref((void*)({}));", code)
                            };
                            format!("{} = {};\n    {}", gm, code, incref)
                        } else {
                            format!("{} = {};", gm, code)
                        };
                        fg.w(&assign);
                        fg.flush_temps(self);
                        body.push_str(&fg.buf);
                        writeln!(
                            c,
                            "void fang_ginit_{m}_{n}(void) {{\n{b}\n}}",
                            m = mname,
                            n = name,
                            b = body
                        )
                        .unwrap();
                        ginit_calls
                            .push_str(&format!("    fang_ginit_{}_{}();\n", mname, name));
                    }
                }
            }
            writeln!(c, "void fang_ginit_{m}(void) {{", m = mname).unwrap();
            writeln!(c, "    fang_init_strings_{m}();", m = mname).unwrap();
            c.push_str(&ginit_calls);
            writeln!(c, "}}").unwrap();
            mods.push(ModUnit {
                module: mname.clone(),
                c,
                h,
            });
        }

        // Entry TU.
        let eu = self.sources.iter().find(|u| u.name == entry).unwrap();
        let mut c = String::new();
        writeln!(c, "/* generated by fang; do not edit */").unwrap();
        writeln!(c, "#include \"fang_rt.h\"").unwrap();
        for mname in &order {
            if mname != entry {
                writeln!(c, "#include \"fang_mod_{m}.h\"", m = mname).unwrap();
            }
        }
        // entry extern headers
        let mut mheaders: BTreeSet<String> = BTreeSet::new();
        for item in &eu.module.items {
            if let Item::ExternFn { header, .. } = item {
                let ht = header.trim();
                if !ht.is_empty()
                    && ht.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
                {
                    mheaders.insert(ht.to_string());
                }
            }
        }
        for hh in &mheaders {
            writeln!(c, "#include <{}>", hh).unwrap();
        }
        writeln!(c).unwrap();
        let e_insts = self.module_insts(entry);
        let saved = std::mem::take(&mut self.out);
        self.emit_forward_typedefs(&e_insts);
        c.push_str(&self.out);
        self.out = saved;
        writeln!(c).unwrap();
        // entry-owned struct defs (inline like the single-TU path)
        let saved = std::mem::take(&mut self.out);
        let mut owned: Vec<String> = self
            .a
            .structs
            .values()
            .filter(|si| si.module == entry)
            .map(|si| si.mangled.clone())
            .collect();
        owned.sort();
        for mangled in &owned {
            self.emit_struct_def(mangled);
        }
        c.push_str(&self.out);
        self.out = saved;
        writeln!(c).unwrap();
        let saved = std::mem::take(&mut self.out);
        self.emit_stamps(&e_insts)?;
        c.push_str(&self.out);
        self.out = saved;
        writeln!(c).unwrap();
        // entry pool
        let mut pool: HashMap<String, String> = HashMap::new();
        let mut pool_order: Vec<String> = Vec::new();
        collect_strs(&eu.module, &mut pool, &mut pool_order);
        for (i, _t) in pool_order.iter().enumerate() {
            writeln!(c, "static fang_str* L{};", i).unwrap();
        }
        writeln!(c, "static void fang_init_strings_{e}(void) {{", e = entry).unwrap();
        for (i, text) in pool_order.iter().enumerate() {
            writeln!(
                c,
                "    L{i} = fang_str_new(\"{text}\", {len}); L{i}->rc = FANG_RC_STATIC;",
                i = i,
                text = c_escape(text),
                len = text.chars().count()
            )
            .unwrap();
        }
        writeln!(c, "}}").unwrap();
        // entry globals
        let mut has_globals = false;
        for item in &eu.module.items {
            if let Item::Const { name, .. } = item {
                let gm = format!("g_{}_{}", entry, name);
                if let Some(ty) = self.a.globals.get(&gm) {
                    writeln!(c, "static {} {};", self.ctype(ty), gm).unwrap();
                    has_globals = true;
                }
            }
        }
        if has_globals {
            writeln!(c).unwrap();
        }
        // entry prototypes
        let mut entry_fns: Vec<&crate::sema::FnInfo> = self
            .a
            .fns
            .values()
            .filter(|fi| fi.module == entry && !fi.is_extern)
            .collect();
        entry_fns.sort_by(|x, y| x.mangled.cmp(&y.mangled));
        for fi in entry_fns {
            let params = self.fn_params(fi);
            let ret = self.ctype_ret(&fi.ret);
            writeln!(c, "static {} {}({});", ret, fi.mangled, params).unwrap();
        }
        writeln!(c).unwrap();
        // entry bodies + extras
        let saved = std::mem::take(&mut self.out);
        *self.extra_decls.borrow_mut() = String::new();
        *self.extra_fns.borrow_mut() = String::new();
        *self.stamped_sigs.borrow_mut() = HashSet::new();
        self.pool = pool.clone();
        self.pool_order = pool_order.clone();
        for item in &eu.module.items {
            if let Item::Fn(f) = item {
                let mangled = match &f.receiver {
                    Some(r) => format!("f_{}_{}_{}", eu.name, r, f.name),
                    None => format!("f_{}_{}", eu.name, f.name),
                };
                let fi = self.a.fns[&mangled].clone();
                self.emit_fn(eu, f, &fi);
            }
        }
        let extra_d = self.extra_decls.borrow().clone();
        let extra_f = self.extra_fns.borrow().clone();
        let extras = format!("{}{}", extra_d, extra_f);
        let bodies = std::mem::replace(&mut self.out, saved);
        c.push_str(&extras);
        writeln!(c).unwrap();
        c.push_str(&bodies);
        writeln!(c).unwrap();
        // entry globals init: one function per global
        let mut entry_ginit_calls = String::new();
        for item in &eu.module.items {
            if let Item::Const { name, value, .. } = item {
                let gm = format!("g_{}_{}", entry, name);
                if self.a.globals.contains_key(&gm) {
                    let mut body = String::new();
                    let path = eu.path.clone();
                    let mut fg = FnGen {
                        module: entry.to_string(),
                        path,
                        a: self.a,
                        captures: HashMap::new(),
                        buf: String::new(),
                        indent: 1,
                        scopes: vec![HashMap::new()],
                        frame: String::new(),
                        field_n: 0,
                        temp_n: 0,
                        locals: Vec::new(),
                        has_temps: Vec::new(),
                        ret: Ty::Void,
                    };
                    let ty = self.a.globals.get(&gm).cloned().unwrap_or(Ty::Err);
                    let (code, _t, fresh) = fg.gen_expr(value, self);
                    let assign = if self.is_rc(&ty) {
                        let incref = if fresh { String::new() } else {
                            format!("fang_incref((void*)({}));", code)
                        };
                        format!("{} = {};\n    {}", gm, code, incref)
                    } else {
                        format!("{} = {};", gm, code)
                    };
                    fg.w(&assign);
                    fg.flush_temps(self);
                    body.push_str(&fg.buf);
                    writeln!(
                        c,
                        "static void fang_ginit_{e}_{n}(void) {{\n{b}\n}}",
                        e = entry,
                        n = name,
                        b = body
                    )
                    .unwrap();
                    entry_ginit_calls
                        .push_str(&format!("    fang_ginit_{}_{}();\n", entry, name));
                }
            }
        }
        if has_globals {
            writeln!(c, "static void fang_ginit_{e}(void) {{", e = entry).unwrap();
            c.push_str(&entry_ginit_calls);
            writeln!(c, "}}").unwrap();
        }
        // main
        writeln!(
            c,
            "int main(int argc, char** argv) {{\n    fang_os_init(argc, argv);\n    fang_init_strings_{e}();",
            e = entry
        )
        .unwrap();
        for mname in &order {
            if mname != entry {
                writeln!(c, "    fang_ginit_{m}();", m = mname).unwrap();
            }
        }
        if has_globals {
            writeln!(c, "    fang_ginit_{e}();", e = entry).unwrap();
        }
        writeln!(c, "    f_{e}_main();\n    return 0;\n}}", e = entry).unwrap();

        Ok(Program {
            mods,
            entry_c: c,
            ginit_order: order,
        })
    }

    fn run(&mut self, entry: &str) -> Result<String, String> {
        // Dead-module elimination: BFS over imports AND recorded call
        // edges from the entry. Method sugar (`s.find(..)`) resolves to
        // str-module functions without an import, so the call map is
        // needed too. Only reachable modules are emitted.
        let mut reach = HashSet::new();
        let mut queue = vec![entry.to_string()];
        while let Some(m) = queue.pop() {
            if !reach.insert(m.clone()) {
                continue;
            }
            if let Some(imports) = self.a.imports.get(&m) {
                for im in imports {
                    queue.push(im.clone());
                }
            }
            for ((cm, _), target) in &self.a.call {
                if cm != &m {
                    continue;
                }
                let callee_module = match target {
                    crate::sema::CallTarget::Fn(mangled)
                    | crate::sema::CallTarget::FnSelf(mangled) => {
                        self.a.fns.get(mangled).map(|fi| fi.module.clone())
                    }
                    crate::sema::CallTarget::StructNew(mangled) => self
                        .a
                        .structs
                        .get(mangled)
                        .map(|si| si.module.clone()),
                    _ => None,
                };
                if let Some(cm2) = callee_module {
                    if !reach.contains(&cm2) {
                        queue.push(cm2);
                    }
                }
            }
        }
        self.reachable = reach;

        // Collect extern headers (sanitized) and pooled strings.
        for u in self.sources {
            if !self.reachable.contains(&u.name) {
                continue;
            }
            for item in &u.module.items {
                if let Item::ExternFn { header, .. } = item {
                    let h = header.trim();
                    if h.is_empty()
                        || !h
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
                    {
                        return Err(format!(
                            "extern header '{}' is not a plain header name (letters, digits, _ . / - only)",
                            header
                        ));
                    }
                    self.used_headers.insert(h.to_string());
                }
            }
            collect_strs(&u.module, &mut self.pool, &mut self.pool_order);
        }

        // Populate pool name map.
        for (i, _) in self.pool_order.iter().enumerate() {
            let _ = i;
        }
        let order_clone = self.pool_order.clone();
        for (i, text) in order_clone.iter().enumerate() {
            self.pool.insert(text.clone(), format!("L{}", i));
        }

        // Prelude.
        self.out.clear();
        if !self.opts.freestanding {
            writeln!(self.out, "/* generated by fang; do not edit */").unwrap();
            writeln!(self.out, "#include \"fang_rt.h\"").unwrap();
        } else {
            writeln!(self.out, "/* generated by fang (freestanding); do not edit */").unwrap();
            writeln!(self.out, "#include <stdint.h>").unwrap();
            writeln!(self.out, "#include <stdbool.h>").unwrap();
        }
        for h in &self.used_headers {
            writeln!(self.out, "#include <{}>", h).unwrap();
        }
        writeln!(self.out).unwrap();

        // Collection types used as struct fields must be declared
        // before the struct definitions; the macro stamps later
        // complete them.
        let insts0 = self.collect_instantiations();
        for t in &insts0 {
            match t {
                Ty::List(e) => {
                    let es = self.sfx(e);
                    writeln!(self.out, "typedef struct fang_list_{0} fang_list_{0};", es).unwrap();
                }
                Ty::Dict(k, v) => {
                    let ks = self.sfx(k);
                    let vs = self.sfx(v);
                    writeln!(self.out, "typedef struct fang_dict_{0}_{1} fang_dict_{0}_{1};", ks, vs).unwrap();
                    writeln!(self.out, "typedef struct fang_dslot_{0}_{1} fang_dslot_{0}_{1};", ks, vs).unwrap();
                }
                Ty::Tuple(a, b) => {
                    let asx = self.sfx(a);
                    let bsx = self.sfx(b);
                    writeln!(self.out, "typedef struct fang_tuple_{0}_{1} fang_tuple_{0}_{1};", asx, bsx).unwrap();
                }
                _ => {}
            }
        }
        writeln!(self.out).unwrap();

        // Module-level mutable globals: declared at file scope,
        // initialized by __fang_globals_init before main runs.
        for (gm, ty) in &self.a.globals {
            if let Some(m) = gm.strip_prefix("g_") {
                if !self
                    .reachable
                    .iter()
                    .any(|r| m.starts_with(&format!("{}_", r)))
                {
                    continue;
                }
            }
            writeln!(self.out, "static {} {};", self.ctype(ty), gm).unwrap();
        }
        if !self.a.globals.is_empty() {
            writeln!(self.out).unwrap();
        }

        // Struct definitions, dependency ordered.
        let order = struct_order(self.a);
        for mangled in &order {
            let m = self
                .a
                .structs
                .get(mangled)
                .map(|si| si.module.clone())
                .unwrap_or_default();
            if !self.reachable.contains(&m) {
                continue;
            }
            self.emit_struct_def(mangled);
        }
        writeln!(self.out).unwrap();

        // Collection stamps: tuples, lists, dicts.
        let insts = self.collect_instantiations();
        for t in &insts {
            if let Ty::Tuple(x, y) = t {
                let sfx = self.sfx(&Ty::Tuple(x.clone(), y.clone()));
                writeln!(
                    self.out,
                    "FANG_TUPLE_DEFINE({}, {}, {}, {})",
                    self.sfx(x),
                    self.sfx(y),
                    self.ctype(x),
                    self.ctype(y)
                )
                .map_err(|e| e.to_string())?;
                let _ = sfx;
            }
        }
        for t in &insts {
            if let Ty::List(e) = t {
                self.emit_list_stamp(e);
            }
        }
        for t in &insts {
            if let Ty::Dict(k, v) = t {
                self.emit_dict_stamp(k, v);
            }
        }
        writeln!(self.out).unwrap();

        // String pool: the init function is always emitted because main
        // always calls it; an empty pool yields an empty body.
        if !self.opts.freestanding {
            for (i, _unused_text) in self.pool_order.iter().enumerate() {
                writeln!(self.out, "static fang_str* L{};", i).unwrap();
            }
            writeln!(
                self.out,
                "static void fang_init_strings(void) {{"
            )
            .unwrap();
            for (i, text) in self.pool_order.iter().enumerate() {
                writeln!(
                    self.out,
                    "    L{} = fang_str_new(\"{}\", {}); L{}->rc = FANG_RC_STATIC;",
                    i,
                    c_escape(text),
                    text.chars().count(),
                    i
                )
                .unwrap();
            }
            writeln!(self.out, "}}").unwrap();
            writeln!(self.out).unwrap();
        }

        // Prototypes.
        let mut fn_order: Vec<&crate::sema::FnInfo> = self
            .a
            .fns
            .values()
            .filter(|fi| self.reachable.contains(&fi.module))
            .collect();
        fn_order.sort_by(|x, y| x.mangled.cmp(&y.mangled));
        for fi in &fn_order {
            if fi.is_extern {
                continue;
            }
            let params = self.fn_params(fi);
            let ret = self.ctype_ret(&fi.ret);
            writeln!(self.out, "static {} {}({});", ret, fi.mangled, params).unwrap();
        }
        writeln!(self.out).unwrap();
        // Extras (lambda bodies, trampolines) are spliced here at the end.
        writeln!(self.out, "/*__FANG_EXTRAS__*/").unwrap();

        // Bodies.
        for u in self.sources {
            if !self.reachable.contains(&u.name) {
                continue;
            }
            for item in &u.module.items {
                if let Item::Fn(f) = item {
                    let mangled = match &f.receiver {
                        Some(r) => format!("f_{}_{}_{}", u.name, r, f.name),
                        None => format!("f_{}_{}", u.name, f.name),
                    };
                    let fi = self.a.fns[&mangled].clone();
                    self.emit_fn(u, f, &fi);
                }
            }
        }

        // Lambda bodies, trampolines, and adapters discovered during
        // function generation: spliced before the bodies that use them.
        let extra_d = self.extra_decls.borrow().clone();
        let extra_f = self.extra_fns.borrow().clone();
        let extras = format!("{}{}", extra_d, extra_f);
        self.out = self.out.replacen("/*__FANG_EXTRAS__*/", &extras, 1);

        // Global initializers: one function assigning every
        // module-level mutable value, called before the entry main.
        if !self.a.globals.is_empty() {
            let mut inits = String::new();
            // (module, name, value) in module item order
            let mut jobs: Vec<(String, String, Expr)> = Vec::new();
            for u in self.sources {
                let mname = &u.name;
                if !self.reachable.contains(mname) {
                    continue;
                }
                for item in &u.module.items {
                    if let Item::Const { name, value, .. } = item {
                        let gm = format!("g_{}_{}", mname, name);
                        if self.a.globals.contains_key(&gm) {
                            jobs.push((mname.clone(), name.clone(), value.clone()));
                        }
                    }
                }
            }
            for (mname, name, value) in jobs {
                let gm = format!("g_{}_{}", mname, name);
                let path = self
                    .sources
                    .iter()
                    .find(|u| u.name == mname)
                    .map(|u| u.path.clone())
                    .unwrap_or_default();
                let mut fg = FnGen {
                    module: mname.clone(),
                    path,
                    a: self.a,
                    captures: HashMap::new(),
                    buf: String::new(),
                    indent: 1,
                    scopes: vec![HashMap::new()],
                    frame: String::new(),
                    field_n: 0,
                    temp_n: 0,
                    locals: Vec::new(),
                    has_temps: Vec::new(),
                    ret: Ty::Void,
                };
                let ty = self.a.globals.get(&gm).cloned().unwrap_or(Ty::Err);
                let (code, _t, fresh) = fg.gen_expr(&value, self);
                let assign = if self.is_rc(&ty) {
                    let incref = if fresh { String::new() } else {
                        format!("fang_incref((void*)({}));", code)
                    };
                    format!("{} = {};\n    {}", gm, code, incref)
                } else {
                    format!("{} = {};", gm, code)
                };
                fg.w(&assign);
                fg.flush_temps(self);
                let body = std::mem::take(&mut fg.buf);
                inits.push_str(&format!(
                    "static void __fang_ginit_{}_{}(void) {{\n{}\n}}\n\n",
                    mname, name, body
                ));
            }
            self.out.push_str(&inits);
            writeln!(self.out, "static void __fang_globals_init(void) {{").unwrap();
            for u in self.sources {
                let mname = &u.name;
                if !self.reachable.contains(mname) {
                    continue;
                }
                for item in &u.module.items {
                    if let Item::Const { name, .. } = item {
                        let gm = format!("g_{}_{}", mname, name);
                        if self.a.globals.contains_key(&gm) {
                            writeln!(self.out, "    __fang_ginit_{}_{}();", mname, name).unwrap();
                        }
                    }
                }
            }
            writeln!(self.out, "}}").unwrap();
            writeln!(self.out).unwrap();
        }

        // Entry wrapper.
        let entry_main = format!("f_{}_main", entry);
        if self.opts.freestanding {
            writeln!(
                self.out,
                "void fang_kernel_main(void) {{ {}(); }}",
                entry_main
            )
            .unwrap();
        } else {
            let ginit = if self.a.globals.is_empty() {
                String::new()
            } else {
                "    __fang_globals_init();\n".to_string()
            };
            writeln!(
                self.out,
                "int main(int argc, char** argv) {{\n    fang_os_init(argc, argv);\n    fang_init_strings();\n{}    {}();\n    return 0;\n}}",
                ginit,
                entry_main
            )
            .unwrap();
        }

        Ok(self.out.clone())
    }

    // ---------------- types ----------------

    fn sfx(&self, t: &Ty) -> String {
        match t {
            Ty::Int => "i64".into(),
            Ty::Float => "f64".into(),
            Ty::Bool => "b8".into(),
            Ty::Str => "str".into(),
            Ty::Ptr => "ptr".into(),
            Ty::List(e) => format!("l_{}", self.sfx(e)),
            Ty::Dict(k, v) => format!("d_{}_{}", self.sfx(k), self.sfx(v)),
            Ty::Tuple(a, b) => format!("t_{}_{}", self.sfx(a), self.sfx(b)),
            Ty::Struct(m) => m.clone(),
            Ty::Func(_) | Ty::Sig(..) | Ty::Any | Ty::Void | Ty::Err => "v".into(),
        }
    }

    fn ctype(&self, t: &Ty) -> String {
        match t {
            Ty::Int => "int64_t".into(),
            Ty::Float => "double".into(),
            Ty::Bool => "bool".into(),
            Ty::Str => "fang_str*".into(),
            Ty::Ptr => "void*".into(),
            Ty::List(e) => format!("fang_list_{}*", self.sfx(e)),
            Ty::Dict(k, v) => format!("fang_dict_{}_{}*", self.sfx(k), self.sfx(v)),
            Ty::Tuple(a, b) => format!("fang_tuple_{}_{}", self.sfx(a), self.sfx(b)),
            Ty::Struct(m) => format!("fang_{}", m),
            Ty::Sig(..) => "fang_closure*".into(),
            Ty::Func(_) => "fang_closure*".into(),
            Ty::Any => "fang_str*".into(),
            Ty::Void | Ty::Err => "void".into(),
        }
    }

    fn ctype_ret(&self, t: &Ty) -> String {
        self.ctype(t)
    }

    fn fn_params(&self, fi: &crate::sema::FnInfo) -> String {
        let mut ps = Vec::new();
        for (i, p) in fi.params.iter().enumerate() {
            ps.push(format!("{} p{}", self.ctype(&p.ty), i));
        }
        if ps.is_empty() {
            "void".into()
        } else {
            ps.join(", ")
        }
    }

    fn is_rc(&self, t: &Ty) -> bool {
        matches!(t, Ty::Str | Ty::List(_) | Ty::Dict(_, _) | Ty::Func(_) | Ty::Sig(..))
    }

    fn decref_stmt(&self, expr: &str, t: &Ty) -> String {
        match t {
            Ty::Str => format!("fang_str_decref({});", expr),
            Ty::Func(_) | Ty::Sig(..) => format!("fang_closure_decref({});", expr),
            Ty::List(e) => format!("fang_list_{}_decref({});", self.sfx(e), expr),
            Ty::Dict(k, v) => {
                format!("fang_dict_{}_{}_decref({});", self.sfx(k), self.sfx(v), expr)
            }
            _ => String::new(),
        }
    }

    /// Field-wise incref for struct copies.
    fn struct_field_incref(&self, prefix: &str, mangled: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(si) = self.a.structs.get(mangled) {
            for (fname, ft) in &si.fields {
                if self.is_rc(ft) {
                    out.push(format!("fang_incref((void*)({}.{}));", prefix, fname));
                }
            }
        }
        out
    }

    fn struct_field_decref(&self, prefix: &str, mangled: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(si) = self.a.structs.get(mangled) {
            for (fname, ft) in &si.fields {
                match ft {
                    Ty::Str => out.push(format!("fang_str_decref({}.{});", prefix, fname)),
                    Ty::List(e) => {
                        out.push(format!("fang_list_{}_decref({}.{});", self.sfx(e), prefix, fname))
                    }
                    Ty::Dict(k, v) => out.push(format!(
                        "fang_dict_{}_{}_decref({}.{});",
                        self.sfx(k),
                        self.sfx(v),
                        prefix,
                        fname
                    )),
                    _ => {}
                }
            }
        }
        out
    }

    // ---------------- struct emission ----------------

    fn emit_struct_def(&mut self, mangled: &str) {
        let Some(si) = self.a.structs.get(mangled) else {
            return;
        };
        let sname = format!("fang_{}", mangled);
        writeln!(self.out, "typedef struct {} {{", sname).unwrap();
        for (fname, ft) in &si.fields {
            // Collection fields are rejected at sema; structs, str, ptr and
            // primitives are emitted directly. str needs the runtime header.
            let cty = match ft {
                Ty::Str => {
                    if self.opts.freestanding {
                        "void*".to_string()
                    } else {
                        "fang_str*".into()
                    }
                }
                other => self.ctype(other),
            };
            writeln!(self.out, "    {} {};", cty, fname).unwrap();
        }
        writeln!(self.out, "}} {};", sname).unwrap();

        // constructor
        let params: Vec<String> = si
            .fields
            .iter()
            .enumerate()
            .map(|(i, (_, t))| format!("{} a{}", self.ctype(t), i))
            .collect();
        let assigns: Vec<String> = si
            .fields
            .iter()
            .enumerate()
            .map(|(i, (n, _))| format!("s.{} = a{};", n, i))
            .collect();
        let ctor_name = format!("{}_new", sname);
        writeln!(
            self.out,
            "static inline {} {}({}) {{\n    {} s;\n    {}\n    return s;\n}}",
            sname,
            ctor_name,
            params.join(", "),
            sname,
            assigns.join("\n    ")
        )
        .unwrap();

        // equality helper when comparable
        let comparable = si
            .fields
            .iter()
            .all(|(_, t)| matches!(t, Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Ptr));
        if comparable {
            let conds: Vec<String> = si
                .fields
                .iter()
                .map(|(n, t)| match t {
                    Ty::Str => format!("fang_str_eq(a.{}, b.{})", n, n),
                    _ => format!("a.{} == b.{}", n, n),
                })
                .collect();
            let cond = if conds.is_empty() {
                "true".to_string()
            } else {
                conds.join(" && ")
            };
            let eq_name = format!("{}_eq", sname);
            writeln!(
                self.out,
                "static inline bool {}({} a, {} b) {{\n    return {};\n}}",
                eq_name,
                sname,
                sname,
                cond
            )
            .unwrap();
            self.used_eq_helpers.insert(mangled.to_string());
        }
        writeln!(self.out).unwrap();
    }

    fn emit_list_stamp(&mut self, elem: &Ty) {
        let sfx = self.sfx(elem);
        let t = self.ctype(elem);
        let (eq, inc, dec): (String, String, String) = match elem {
            Ty::Int => ("FANG_EQ_I64".into(), "FANG_NOOP".into(), "FANG_NOOP".into()),
            Ty::Float => ("FANG_EQ_F64".into(), "FANG_NOOP".into(), "FANG_NOOP".into()),
            Ty::Bool => ("FANG_EQ_B8".into(), "FANG_NOOP".into(), "FANG_NOOP".into()),
            Ty::Ptr => ("FANG_EQ_PTR".into(), "FANG_NOOP".into(), "FANG_NOOP".into()),
            Ty::Str => ("FANG_EQ_STR".into(), "FANG_INC_STR".into(), "FANG_DEC_STR".into()),
            Ty::Struct(m) => (
                format!("fang_{}_eq", m),
                "FANG_NOOP".into(),
                "FANG_NOOP".into(),
            ),
            Ty::Tuple(..) => ("FANG_EQ_NEVER".into(), "FANG_NOOP".into(), "FANG_NOOP".into()),
            _ => ("FANG_EQ_PTR".into(), "FANG_NOOP".into(), "FANG_NOOP".into()),
        };
        if self.opts.freestanding {
            return;
        }
        writeln!(
            self.out,
            "FANG_LIST_DEFINE({}, {}, {}, {}, {})",
            sfx, t, eq, inc, dec
        )
        .unwrap();
        writeln!(self.out).unwrap();
    }

    fn emit_dict_stamp(&mut self, k: &Ty, v: &Ty) {
        if self.opts.freestanding {
            return;
        }
        let ks = self.sfx(k);
        let vs = self.sfx(v);
        let (khash, keq, dec_k, kinc): (String, String, String, String) = match k {
            Ty::Int => ("FANG_HASH_I64".into(), "FANG_EQ_I64".into(), "FANG_NOOP".into(), "FANG_NOOP".into()),
            _ => ("FANG_HASH_STR".into(), "FANG_EQ_STR".into(), "FANG_DEC_STR".into(), "FANG_INC_STR".into()),
        };
        let (dec_v, vinc): (String, String) = match v {
            Ty::Str => ("FANG_DEC_STR".into(), "FANG_INC_STR".into()),
            _ => ("FANG_NOOP".into(), "FANG_NOOP".into()),
        };
        writeln!(
            self.out,
            "FANG_DICT_DEFINE({}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
            ks,
            vs,
            self.ctype(k),
            self.ctype(v),
            khash,
            keq,
            dec_k,
            dec_v,
            kinc,
            vinc
        )
        .unwrap();
        writeln!(self.out).unwrap();
    }

    /// Every collection type reachable from signatures, locals, and fields.
    fn collect_instantiations(&self) -> BTreeSet<Ty> {
        let mut out: BTreeSet<Ty> = BTreeSet::new();
        let mut work: Vec<Ty> = Vec::new();
        for fi in self.a.fns.values() {
            if !self.reachable.contains(&fi.module) {
                continue;
            }
            for p in &fi.params {
                work.push(p.ty.clone());
            }
            work.push(fi.ret.clone());
        }
        for si in self.a.structs.values() {
            if !self.reachable.contains(&si.module) {
                continue;
            }
            for (_, t) in &si.fields {
                work.push(t.clone());
            }
        }
        work.extend(
            self.a
                .ty
                .iter()
                .filter(|((m, _), _)| self.reachable.contains(m.as_str()))
                .map(|(_, t)| t.clone()),
        );
        work.extend(
            self.a
                .for_binders
                .iter()
                .filter(|((m, _), _)| self.reachable.contains(m.as_str()))
                .flat_map(|(_, v)| v.iter().cloned()),
        );
        work.extend(
            self.a
                .catch_ty
                .iter()
                .filter(|((m, _), _)| self.reachable.contains(m.as_str()))
                .map(|(_, t)| t.clone()),
        );
        for t in self.a.consts.values() {
            work.push(match t {
                ConstVal::Int(_) => Ty::Int,
                ConstVal::Float(_) => Ty::Float,
                ConstVal::Bool(_) => Ty::Bool,
                ConstVal::Str(_) => Ty::Str,
            });
        }
        while let Some(t) = work.pop() {
            match &t {
                Ty::List(e) => {
                    if out.insert(t.clone()) {
                        work.push((**e).clone());
                    }
                }
                Ty::Dict(k, v) => {
                    if out.insert(t.clone()) {
                        // dict lowering uses keys/values/items helpers
                        let kv = Ty::Tuple(Box::new((**k).clone()), Box::new((**v).clone()));
                        out.insert(Ty::List(Box::new(kv.clone())));
                        out.insert(kv);
                        out.insert(Ty::List(Box::new((**k).clone())));
                        out.insert(Ty::List(Box::new((**v).clone())));
                        work.push((**k).clone());
                        work.push((**v).clone());
                    }
                }
                Ty::Tuple(a, b) if out.insert(t.clone()) => {
                    work.push((**a).clone());
                    work.push((**b).clone());
                }
                _ => {}
            }
        }
        out.retain(|t| matches!(t, Ty::List(_) | Ty::Dict(_, _) | Ty::Tuple(_, _)));
        out
    }

    // ---------------- functions ----------------

    fn emit_fn(&mut self, u: &SourceUnit, f: &FnDecl, fi: &crate::sema::FnInfo) {
        let mut fg = FnGen {
            module: u.name.clone(),
            path: u.path.clone(),
            a: self.a,
            captures: HashMap::new(),
            buf: String::new(),
            indent: 1,
            scopes: vec![HashMap::new()],
            locals: Vec::new(),
            has_temps: Vec::new(),
            frame: String::new(),
            field_n: 0,
            temp_n: 0,
            ret: fi.ret.clone(),
        };
        // Bind params into the frame (longjmp-safe copies), owning refs.
        for (i, p) in fi.params.iter().enumerate() {
            let field = format!("{}__{}", p.name, fg.next_field());
            let inc = if self.is_rc(&p.ty) {
                format!("    fang_incref((void*)(p{}));\n", i)
            } else {
                String::new()
            };
            let struct_inc = if let Ty::Struct(m) = &p.ty {
                self.struct_field_incref(&format!("f.{}", field), m)
                    .iter()
                    .map(|s| format!("    {}\n", s))
                    .collect::<String>()
            } else {
                String::new()
            };
            fg.frame.push_str(&format!("    {} {};\n", self.ctype(&p.ty), field));
            fg.buf.push_str(&format!("    f.{} = p{};\n{}", field, i, inc));
            fg.buf.push_str(&struct_inc);
            let var = Var { field: field.clone(), ty: p.ty.clone() };
            fg.scopes.last_mut().unwrap().insert(p.name.clone(), var.clone());
            fg.locals.push(var);
        }

        emit_body_with_trailing(&mut fg, &f.body, &fi.ret, self);

        let params = self.fn_params(fi);
        let ret = self.ctype_ret(&fi.ret);
        let is_void = matches!(fi.ret, Ty::Void | Ty::Err);
        let ret_decl = if is_void {
            String::new()
        } else {
            format!("    {} __ret;\n", ret)
        };
        // Single exit: every return jumps here; locals are released once.
        let mut cleanup = String::new();
        for v in &fg.locals {
            match &v.ty {
                Ty::Struct(m) => {
                    for cs in self.struct_field_decref(&format!("f.{}", v.field), m) {
                        cleanup.push_str("    ");
                        cleanup.push_str(&cs);
                        cleanup.push('\n');
                    }
                }
                t => {
                    let d = self.decref_stmt(&format!("f.{}", v.field), t);
                    if !d.is_empty() {
                        cleanup.push_str("    ");
                        cleanup.push_str(&d);
                        cleanup.push('\n');
                    }
                }
            }
        }
        let ret_stmt = if is_void {
            "    return;\n".to_string()
        } else {
            "    return __ret;\n".to_string()
        };
        // Zero-init only when the frame has fields; {0} on an empty struct
        // trips a GCC excess-elements warning.
        let init = if fg.frame.trim().is_empty() {
            String::new()
        } else {
            " = {0}".to_string()
        };
        let linkage = if self.fns_external { "" } else { "static " };
        writeln!(
            self.out,
            "{linkage}{} {}({}) {{\n{}    struct {{\n{}    }} f{};\n{}__exit: ;\n{}{}}}",
            ret,
            fi.mangled,
            params,
            ret_decl,
            fg.frame,
            init,
            fg.buf,
            cleanup,
            ret_stmt
        )
        .unwrap();
        writeln!(self.out).unwrap();
    }
}

// ---------------------------------------------------------------------
// Statement/expression generation, one function at a time.
// ---------------------------------------------------------------------

#[derive(Clone)]
struct Var {
    field: String,
    ty: Ty,
}

struct FnGen<'a> {
    module: String,
    path: String,
    a: &'a Analysis,
    /// Lambda body generation: captured name -> C member expression.
    captures: HashMap<String, String>,
    buf: String,
    indent: usize,
    scopes: Vec<HashMap<String, Var>>,
    /// Every local ever declared, cleaned once at __exit.
    locals: Vec<Var>,
    /// Fresh values consumed as borrowed within the current statement.
    has_temps: Vec<(String, Ty)>,
    frame: String,
    field_n: usize,
    temp_n: usize,
    ret: Ty,
}

impl<'a> FnGen<'a> {
    fn next_field(&mut self) -> usize {
        self.field_n += 1;
        self.field_n
    }

    fn w(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("    ");
        }
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    fn raw(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    fn lookup(&self, name: &str) -> Option<&Var> {
        for s in self.scopes.iter().rev() {
            if let Some(v) = s.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn declare(&mut self, name: &str, ty: Ty, g: &Gen) {
        let field = format!("{}__{}", name, self.next_field());
        self.frame.push_str(&format!("    {} {};\n", g.ctype(&ty), field));
        let var = Var { field, ty };
        self.scopes.last_mut().unwrap().insert(name.to_string(), var.clone());
        self.locals.push(var);
    }

    fn flush_temps(&mut self, g: &Gen) {
        let temps = std::mem::take(&mut self.has_temps);
        for (expr, t) in temps {
            let d = g.decref_stmt(&expr, &t);
            if !d.is_empty() {
                self.w(&d);
            }
        }
    }

    // ---------------- statements ----------------

    fn stmts(&mut self, stmts: &[Stmt], g: &Gen) {
        self.scopes.push(HashMap::new());
        for st in stmts {
            self.stmt(st, g);
        }
        self.scopes.pop();
    }

    fn line_of(&mut self, st: &Stmt) {
        // #line maps C diagnostics back to Fang source.
        self.raw(&format!("#line {} \"{}\"\n", st.span.line, self.path));
    }

    fn stmt(&mut self, st: &Stmt, g: &Gen) {
        match &st.kind {
            StmtKind::Comment(_) | StmtKind::Pass => {}
            StmtKind::Asm(code) => {
                self.line_of(st);
                self.w(&format!("__asm__ volatile(\"{}\");", c_escape(code)));
            }
            StmtKind::Break => {
                self.line_of(st);
                self.w("break;");
            }
            StmtKind::Continue => {
                self.line_of(st);
                self.w("continue;");
            }
            StmtKind::Let { binders, value, .. } => {
                self.line_of(st);
                let vt = self.a.ty[&(self.module.clone(), value.span.start)].clone();
                if binders.len() == 2 {
                    if let Ty::Tuple(x, y) = &vt {
                        let (tx, ty2) = ((**x).clone(), (**y).clone());
                        let (vx, vy) = self.gen_tuple_expr(value, g);
                        self.declare(&binders[0], tx.clone(), g);
                        self.declare(&binders[1], ty2.clone(), g);
                        let bx = self.lookup(&binders[0]).unwrap().field.clone();
                        let by = self.lookup(&binders[1]).unwrap().field.clone();
                        self.w(&format!("f.{} = {}.a;", bx, vx));
                        self.w(&format!("f.{} = {}.b;", by, vy));
                        if g.is_rc(&tx) {
                            self.w(&format!("fang_incref((void*)(f.{}));", bx));
                        }
                        if g.is_rc(&ty2) {
                            self.w(&format!("fang_incref((void*)(f.{}));", by));
                        }
                    }
                } else {
                    let (code, _t, fresh) = self.gen_expr(value, g);
                    self.declare(&binders[0], vt.clone(), g);
                    let field = self.lookup(&binders[0]).unwrap().field.clone();
                    self.w(&format!("f.{} = {};", field, code));
                    match &vt {
                        t if g.is_rc(t) && !fresh => {
                            self.w(&format!("fang_incref((void*)(f.{}));", field));
                        }
                        Ty::Struct(m) => {
                            for si in g.struct_field_incref(&format!("f.{}", field), m) {
                                self.w(&si);
                            }
                        }
                        _ => {}
                    }
                }
                self.flush_temps(g);
            }
            StmtKind::TypedAssign { name, ann: _, value } => {
                self.line_of(st);
                let vt = self.a.ty[&(self.module.clone(), value.span.start)].clone();
                let (code, _t, fresh) = self.gen_expr(value, g);
                if self.lookup(name).is_none() {
                    self.declare(name, vt.clone(), g);
                }
                let field = self.lookup(name).unwrap().field.clone();
                match &vt {
                    Ty::Struct(m) => {
                        self.w(&format!("f.{} = {};", field, code));
                        for si in g.struct_field_incref(&format!("f.{}", field), m) {
                            self.w(&si);
                        }
                    }
                    t if g.is_rc(t) => {
                        self.w(&format!("f.{} = {};", field, code));
                        if !fresh {
                            self.w(&format!("fang_incref((void*)(f.{}));", field));
                        }
                    }
                    _ => self.w(&format!("f.{} = {};", field, code)),
                }
                self.flush_temps(g);
            }
            StmtKind::AssignMulti { targets, value } => {
                self.line_of(st);
                let vt = self.a.ty[&(self.module.clone(), value.span.start)].clone();
                let (at, bt) = match &vt {
                    Ty::Tuple(a, b) => ((**a).clone(), (**b).clone()),
                    _ => (Ty::Err, Ty::Err),
                };
                let (code, _t, fresh) = self.gen_expr(value, g);
                let tmp = format!("__mt{}", self.temp());
                let tname = format!(
                    "fang_tuple_{}_{}",
                    g.sfx(&at),
                    g.sfx(&bt)
                );
                self.w(&format!(
                    "{} {} = {};",
                    tname, tmp, code
                ));
                if !fresh && (g.is_rc(&at) || g.is_rc(&bt)) {
                    // borrowed tuple fields: they were materialized already
                }
                for (i, t) in targets.iter().enumerate() {
                    let (part_ty, part) = if i == 0 {
                        (at.clone(), format!("{}.a", tmp))
                    } else {
                        (bt.clone(), format!("{}.b", tmp))
                    };
                    if let LValue::Ident(n) = t {
                        // New names auto-declare with their part type.
                        if self.lookup(n).is_none() {
                            self.declare(n, part_ty.clone(), g);
                        }
                        let v = self.lookup(n).cloned();
                        if let Some(v) = v {
                            let field = v.field.clone();
                            match &v.ty {
                                Ty::Struct(m) => {
                                    for sd in g.struct_field_decref(&format!("f.{}", field), m) {
                                        self.w(&sd);
                                    }
                                    self.w(&format!("f.{} = {};", field, part));
                                    for si in g.struct_field_incref(&format!("f.{}", field), m) {
                                        self.w(&si);
                                    }
                                }
                                tt if g.is_rc(tt) => {
                                    let d = g.decref_stmt(&format!("f.{}", field), tt);
                                    if !d.is_empty() {
                                        self.w(&d);
                                    }
                                    self.w(&format!("f.{} = {};", field, part));
                                    if !g.is_rc(&part_ty) {
                                        let _ = part_ty;
                                    }
                                }
                                _ => self.w(&format!("f.{} = {};", field, part)),
                            }
                        }
                    }
                }
                self.flush_temps(g);
            }
            StmtKind::Assign { target, value } => {
                self.line_of(st);
                let (code, _ty, fresh) = self.gen_expr(value, g);
                match target {
                    LValue::Ident(n) => {
                        // An undeclared name in codegen means sema
                        // auto-declared it: create the frame slot now.
                        // Globals keep their C-level slot instead.
                        let gm = format!("g_{}_{}", self.module, n);
                        if g.a.globals.contains_key(&gm) {
                            let vt = self.a.ty[&(self.module.clone(), value.span.start)].clone();
                            let field = gm.clone();
                            if g.is_rc(&vt) || matches!(vt, Ty::Struct(_)) {
                                let t = format!("__asg{}", self.temp());
                                self.w(&format!("{} {} = {};", g.ctype(&vt), t, code));
                                let d = g.decref_stmt(&field, &vt);
                                if !d.is_empty() {
                                    self.w(&d);
                                }
                                self.w(&format!("{} = {};", field, t));
                                if g.is_rc(&vt) {
                                    self.w(&format!("fang_incref((void*)({}));", field));
                                }
                            } else {
                                self.w(&format!("{} = {};", field, code));
                            }
                            self.flush_temps(g);
                            return;
                        }
                        if self.lookup(n).is_none() {
                            let vt = self.a.ty[&(self.module.clone(), value.span.start)].clone();
                            self.declare(n, vt, g);
                        }
                        let v = self.lookup(n).unwrap().clone();
                        // Fast path for the string-building idiom
                        // `s = s + rhs`: concat_assign consumes the old
                        // reference and appends in place when the string is
                        // uniquely held with spare capacity.
                        if v.ty == Ty::Str {
                            if let ExprKind::Binary(BinOp::Add, l, r) = &value.kind {
                                if let ExprKind::Ident(lname) = &l.kind {
                                    if lname == n {
                                        let (rhs_code, _, _) = self.gen_expr(r, g);
                                        self.w(&format!(
                                            "f.{} = fang_str_concat_assign(f.{}, {});",
                                            v.field, v.field, rhs_code
                                        ));
                                        self.flush_temps(g);
                                        return;
                                    }
                                }
                            }
                        }
                        // The old value is released before the new one is
                        // stored; materialize the RHS first so expressions
                        // reading the old value stay valid.
                        let code = if g.is_rc(&v.ty) || matches!(v.ty, Ty::Struct(_)) {
                            let t = format!("__asg{}", self.temp());
                            self.w(&format!("{} {} = {};", g.ctype(&v.ty), t, code));
                            t
                        } else {
                            code
                        };
                        match &v.ty {
                            Ty::Struct(m) => {
                                for sd in g.struct_field_decref(&format!("f.{}", v.field), m) {
                                    self.w(&sd);
                                }
                                self.w(&format!("f.{} = {};", v.field, code));
                                for si in g.struct_field_incref(&format!("f.{}", v.field), m) {
                                    self.w(&si);
                                }
                            }
                            t if g.is_rc(t) => {
                                let d = g.decref_stmt(&format!("f.{}", v.field), t);
                                if !d.is_empty() {
                                    self.w(&d);
                                }
                                self.w(&format!("f.{} = {};", v.field, code));
                                if !fresh {
                                    self.w(&format!("fang_incref((void*)(f.{}));", v.field));
                                }
                            }
                            _ => self.w(&format!("f.{} = {};", v.field, code)),
                        }
                    }
                    LValue::Index(recv, idx) => {
                        let rt = self.lvalue_recv_ty(recv, g);
                        let r = self.lvalue_code(recv, g);
                        let (i, _, _) = self.gen_expr(idx, g);
                        match &rt {
                            Ty::List(e) => {
                                self.w(&format!(
                                    "fang_list_{}_set({}, {}, {});",
                                    g.sfx(e),
                                    r,
                                    i,
                                    code
                                ));
                            }
                            Ty::Dict(k, v) => {
                                if g.is_rc(k) && matches!(idx.kind, ExprKind::Ident(_)) {
                                    self.w(&format!("fang_incref((void*)({}));", i));
                                }
                                if g.is_rc(v) && !fresh {
                                    self.w(&format!("fang_incref((void*)({}));", code));
                                }
                                self.w(&format!(
                                    "fang_dict_{}_{}_set({}, {}, {});",
                                    g.sfx(k),
                                    g.sfx(v),
                                    r,
                                    i,
                                    code
                                ));
                            }
                            _ => {}
                        }
                    }
                    LValue::Field(recv, name) => {
                        let rt = self.lvalue_recv_ty(recv, g);
                        if let Ty::Struct(m) = rt {
                            let r = self.lvalue_code(recv, g);
                            let ft = g.a.structs[&m]
                                .fields
                                .iter()
                                .find(|(n, _)| n == name)
                                .map(|(_, t)| t.clone())
                                .unwrap_or(Ty::Err);
                            if g.is_rc(&ft) {
                                let d = g.decref_stmt(&format!("{}.{}", r, name), &ft);
                                if !d.is_empty() {
                                    self.w(&d);
                                }
                            }
                            self.w(&format!("{}.{} = {};", r, name, code));
                        }
                    }
                }
                self.flush_temps(g);
            }
            StmtKind::Return(value) => {
                self.line_of(st);
                match value {
                    Some(e) => {
                        let (code, _t, fresh) = self.gen_expr(e, g);
                        if g.is_rc(&self.ret) && !fresh {
                            self.w(&format!("fang_incref((void*)({}));", code));
                        }
                        let rt_clone = self.ret.clone();
                        if let Ty::Struct(m) = &rt_clone {
                            self.w(&format!("{} __r = {};", g.ctype(&rt_clone), code));
                            for si in g.struct_field_incref("__r", m) {
                                self.w(&si);
                            }
                            self.w("__ret = __r; goto __exit;");
                        } else {
                            self.w(&format!("__ret = {}; goto __exit;", code));
                        }
                    }
                    None => self.w("goto __exit;"),
                }
            }
            StmtKind::If { cond, then, elifs, els } => {
                self.line_of(st);
                self.gen_if_chain(cond, then, elifs, els.as_ref(), g);
                self.flush_temps(g);
            }
            StmtKind::While { cond, body } => {
                self.line_of(st);
                // Re-evaluated per iteration; temps scope per iteration.
                self.w("for (;;) {");
                self.indent += 1;
                let (c, _ct) = self.gen_cond(cond, g);
                self.w(&format!("if (!({})) {{", c));
                self.indent += 1;
                self.flush_temps(g);
                self.w("break;");
                self.indent -= 1;
                self.w("}");
                self.stmts(body, g);
                self.flush_temps(g);
                self.indent -= 1;
                self.w("}");
            }
            StmtKind::For { binders, iter, body } => {
                self.line_of(st);
                self.gen_for(binders, iter, body, st, g);
            }
            StmtKind::Try { body, binder, handler } => {
                self.line_of(st);
                self.w("fang_handler __h;");
                self.w("__h.prev = fang_handler_top; fang_handler_top = &__h;");
                self.w("if (setjmp(__h.buf) == 0) {");
                self.indent += 1;
                self.stmts(body, g);
                self.w("fang_handler_top = __h.prev;");
                self.indent -= 1;
                self.w("} else {");
                self.indent += 1;
                self.w("fang_handler_top = __h.prev;");
                if let Some(b) = binder {
                    self.declare(b, Ty::Any, g);
                    let field = self.lookup(b).unwrap().field.clone();
                    self.w(&format!("f.{} = __h.value;", field));
                }
                self.stmts(handler, g);
                self.indent -= 1;
                self.w("}");
            }
            StmtKind::Raise(value) => {
                self.line_of(st);
                // The raised value becomes a string.
                let (code, t, _) = self.gen_expr(value, g);
                let s = match t {
                    Ty::Str => code,
                    Ty::Int => format!("fang_str_from_i64({})", code),
                    Ty::Float => format!("fang_str_from_f64({})", code),
                    Ty::Bool => format!("fang_str_from_bool({})", code),
                    Ty::Ptr => format!("fang_str_from_i64((int64_t)(intptr_t)({}))", code),
                    _ => code,
                };
                self.w(&format!("fang_raise({});", s));
            }
            StmtKind::Expr(e) => {
                self.line_of(st);
                let (code, t, fresh) = self.gen_expr(e, g);
                // The expression's value is discarded; side-effect calls
                // (void fns) must still run.
                if fresh && g.is_rc(&t) {
                    let tmp = format!("__t{}", self.temp());
                    self.w(&format!("{} {} = {};", g.ctype(&t), tmp, code));
                    self.has_temps.push((tmp, t.clone()));
                } else if code != "0" {
                    self.w(&format!("{};", code));
                }
                self.flush_temps(g);
            }
        }
    }

    fn gen_if_chain(
        &mut self,
        cond: &Expr,
        then: &[Stmt],
        elifs: &[(Expr, Block)],
        els: Option<&Block>,
        g: &Gen,
    ) {
        let (c, _ct) = self.gen_cond(cond, g);
        self.w(&format!("if ({}) {{", c));
        self.indent += 1;
        self.stmts(then, g);
        self.indent -= 1;
        for (ec, eb) in elifs {
            // Bool conditions never materialize temporaries, so the elif
            // condition is safe inline in `else if`.
            let (c2, _) = self.gen_cond(ec, g);
            self.w(&format!("}} else if ({}) {{", c2));
            self.indent += 1;
            self.stmts(eb, g);
            self.indent -= 1;
        }
        if let Some(eb) = els {
            self.w("} else {");
            self.indent += 1;
            self.stmts(eb, g);
            self.indent -= 1;
        }
        self.w("}");
    }

    fn gen_cond(&mut self, e: &Expr, g: &Gen) -> (String, Ty) {
        let (code, t, fresh) = self.gen_expr(e, g);
        // Collections and strings in condition position: nonempty is true.
        match &t {
            Ty::Str => {
                return (format!("(({})->len > 0)", code), Ty::Bool);
            }
            Ty::List(_) => {
                return (format!("(({})->len > 0)", code), Ty::Bool);
            }
            Ty::Dict(_, _) => {
                return (format!("(({})->len > 0)", code), Ty::Bool);
            }
            _ => {}
        }
        if fresh && g.is_rc(&t) {
            // materialize so the value survives until the branch closes
            let name = format!("__t{}", self.temp());
            self.w(&format!("{} {} = {};", g.ctype(&t), name, code));
            self.has_temps.push((name.clone(), t.clone()));
            (name, t)
        } else {
            (code, t)
        }
    }

    fn gen_tuple_expr(&mut self, e: &Expr, g: &Gen) -> (String, String) {
        // Returns C lvalue expressions for a and b of a tuple value.
        let (code, t, _fresh) = self.gen_expr(e, g);
        let tmp = format!("__tup{}", self.temp());
        self.w(&format!("{} {} = {};", g.ctype(&t), tmp, code));
        if g.is_rc(&t) {
            self.has_temps.push((tmp.clone(), t.clone()));
        }
        (tmp.clone(), tmp)
    }

    fn gen_for(
        &mut self,
        binders: &[String],
        iter: &Iter,
        body: &[Stmt],
        st: &Stmt,
        g: &Gen,
    ) {
        let binder_tys = self
            .a
            .for_binders
            .get(&(self.module.clone(), st.span.start))
            .cloned()
            .unwrap_or_default();
        match iter {
            Iter::Range { start, end, inclusive } => {
                let (s, _, _) = self.gen_expr(start, g);
                let (e, _, _) = self.gen_expr(end, g);
                self.declare(&binders[0], Ty::Int, g);
                let field = self.lookup(&binders[0]).unwrap().field.clone();
                self.scopes.push(HashMap::new());
                let cmp = if *inclusive { "<=" } else { "<" };
                self.w(&format!(
                    "for (int64_t __i = {}, __e = {}; __i {} __e; __i++) {{",
                    s, e, cmp
                ));
                self.indent += 1;
                self.w(&format!("f.{} = __i;", field));
                self.stmts(body, g);
                self.indent -= 1;
                self.w("}");
                self.scopes.pop();
            }
            Iter::Expr(ie) => {
                let it = self.a.ty[&(self.module.clone(), ie.span.start)].clone();
                // dict iteration lowers to keys()
                let list_ty = match &it {
                    Ty::Dict(k, _) => Ty::List(k.clone()),
                    other => other.clone(),
                };
                let elem = match &list_ty {
                    Ty::List(e) => (**e).clone(),
                    _ => Ty::Err,
                };
                let (code, _, fresh) = self.gen_expr(ie, g);
                let list_expr = match &it {
                    Ty::Dict(k, v) => format!(
                        "fang_dict_{}_{}_keys({})",
                        g.sfx(k),
                        g.sfx(v),
                        code
                    ),
                    _ => code.clone(),
                };
                let it_var = format!("__it{}", self.temp());
                self.w(&format!(
                    "{} {} = {};",
                    g.ctype(&list_ty),
                    it_var,
                    list_expr
                ));
                self.scopes.push(HashMap::new());
                self.w(&format!("for (int64_t __j = 0; __j < {}->len; __j++) {{", it_var));
                self.indent += 1;
                if binders.len() == 2 {
                    if let Ty::Tuple(ta, tb) = &elem {
                        let tname = format!(
                            "fang_tuple_{}_{}",
                            g.sfx(ta),
                            g.sfx(tb)
                        );
                        self.w(&format!(
                            "{} __p = {}->elems[__j];",
                            tname, it_var
                        ));
                        for (i, b) in binders.iter().enumerate() {
                            let bt = binder_tys.get(i).cloned().unwrap_or(Ty::Err);
                            self.declare(b, bt.clone(), g);
                            let field = self.lookup(b).unwrap().field.clone();
                            // Released at the end of each body below.
                            let part = if i == 0 { "__p.a" } else { "__p.b" };
                            self.w(&format!("f.{} = {};", field, part));
                            if g.is_rc(&bt) {
                                self.w(&format!("fang_incref((void*)(f.{}));", field));
                            }
                        }
                    }
                } else {
                    let b = &binders[0];
                    self.declare(b, elem.clone(), g);
                    let field = self.lookup(b).unwrap().field.clone();
                    // The binder is released at the end of each body
                    // below; no pre-assignment decref here.
                    self.w(&format!("f.{} = {}->elems[__j];", field, it_var));
                    match &elem {
                        t if g.is_rc(t) => {
                            self.w(&format!("fang_incref((void*)(f.{}));", field));
                        }
                        Ty::Struct(m) => {
                            for s in g.struct_field_incref(&format!("f.{}", field), m) {
                                self.w(&s);
                            }
                        }
                        _ => {}
                    }
                }
                self.stmts(body, g);
                // per-iteration binder cleanup
                for b in binders {
                    if let Some(v) = self.scopes.last().unwrap().get(b) {
                        let field = v.field.clone();
                        match &v.ty {
                            Ty::Struct(m) => {
                                for s in g.struct_field_decref(&format!("f.{}", field), m) {
                                    self.w(&s);
                                }
                            }
                            t => {
                                let d = g.decref_stmt(&format!("f.{}", field), t);
                                if !d.is_empty() {
                                    self.w(&d);
                                }
                            }
                        }
                    }
                }
                self.indent -= 1;
                self.w("}");
                // The binder slot still points at the last iterated
                // value, which the per-iteration cleanup already
                // released. Clear it so the function-exit cleanup is a
                // no-op instead of a second decref.
                for b in binders {
                    if let Some(v) = self.scopes.last().unwrap().get(b) {
                        let field = v.field.clone();
                        match &v.ty {
                            t if g.is_rc(t) => {
                                self.w(&format!("f.{} = NULL;", field));
                            }
                            Ty::Struct(m) => {
                                let cty = g.ctype(&Ty::Struct(m.clone()));
                                self.w(&format!("f.{} = ({}){{0}};", field, cty));
                            }
                            _ => {}
                        }
                    }
                }
                // The iterator is ours to release only when the expression
                // produced a fresh list (items(), literals, slices); a
                // variable read stays owned by the variable.
                if fresh || matches!(it, Ty::Dict(_, _)) {
                    let d = g.decref_stmt(&it_var, &list_ty);
                    if !d.is_empty() {
                        self.w(&d);
                    }
                }
                self.scopes.pop();
            }
        }
    }

    fn temp(&mut self) -> usize {
        self.temp_n += 1;
        self.temp_n
    }

    fn lvalue_code(&mut self, lv: &LValue, g: &Gen) -> String {
        match lv {
            LValue::Ident(n) => {
                if let Some(v) = self.lookup(n) {
                    return format!("f.{}", v.field);
                }
                let gm = format!("g_{}_{}", self.module, n);
                if g.a.globals.contains_key(&gm) {
                    return gm;
                }
                format!("f.{}", String::new())
            }
            LValue::Index(recv, idx) => {
                let base = self.lvalue_code(recv, g);
                let (i, _, _) = self.gen_expr(idx, g);
                let rt = self.lvalue_recv_ty(recv, g);
                match rt {
                    Ty::List(e) => format!("fang_list_{}_get({}, {})", g.sfx(&e), base, i),
                    Ty::Dict(k, v) => format!(
                        "fang_dict_{}_{}_get({}, {}, NULL)",
                        g.sfx(&k),
                        g.sfx(&v),
                        base,
                        i
                    ),
                    _ => base,
                }
            }
            LValue::Field(recv, name) => {
                let base = self.lvalue_code(recv, g);
                format!("{}.{}", base, name)
            }
        }
    }

    fn lvalue_recv_ty(&mut self, lv: &LValue, g: &Gen) -> Ty {
        match lv {
            LValue::Ident(n) => {
                if let Some(v) = self.lookup(n) {
                    return v.ty.clone();
                }
                let gm = format!("g_{}_{}", self.module, n);
                g.a.globals.get(&gm).cloned().unwrap_or(Ty::Err)
            }
            LValue::Index(recv, _) => {
                let rt = self.lvalue_recv_ty(recv, g);
                match rt {
                    Ty::List(e) => *e,
                    Ty::Dict(_, v) => *v,
                    _ => Ty::Err,
                }
            }
            LValue::Field(recv, name) => {
                let rt = self.lvalue_recv_ty(recv, g);
                if let Ty::Struct(m) = rt {
                    if let Some(si) = g.a.structs.get(&m) {
                        if let Some((_, ft)) = si.fields.iter().find(|(n, _)| n == name) {
                            return ft.clone();
                        }
                    }
                }
                Ty::Err
            }
        }
    }

    // ---------------- expressions ----------------

    /// Generate an expression. Returns (C text, type, fresh) where fresh
    /// means the caller owns one reference for RC types.
    fn gen_expr_borrowed(&mut self, e: &Expr, g: &Gen) -> (String, Ty, bool) {
        self.gen_expr(e, g)
    }

    /// Signature of a function value by key.
    fn sig_of(&self, key: &str, g: &Gen) -> (Vec<Ty>, Ty) {
        if let Some(ci) = g.a.closures.get(key) {
            (ci.params.clone(), ci.ret.clone())
        } else if let Some(fi) = g.a.fns.get(key) {
            (fi.params.iter().map(|p| p.ty.clone()).collect(), fi.ret.clone())
        } else {
            (Vec::new(), Ty::Err)
        }
    }

    /// Cleanup for a lambda body frame: releases locals + params.
    fn cleanup_stmts(&self, g: &Gen) -> String {
        let mut out = String::new();
        for scope in self.scopes.iter().rev() {
            for v in scope.values() {
                match &v.ty {
                    Ty::Struct(m) => {
                        for s in g.struct_field_decref(&format!("f.{}", v.field), m) {
                            out.push_str("    ");
                            out.push_str(&s);
                            out.push('\n');
                        }
                    }
                    t => {
                        let d = g.decref_stmt(&format!("f.{}", v.field), t);
                        if !d.is_empty() {
                            out.push_str("    ");
                            out.push_str(&d);
                            out.push('\n');
                        }
                    }
                }
            }
        }
        out
    }

    /// A named function used as a value: wrap it in a zero-capture closure.
    fn gen_named_fn_value(&mut self, key: &str, g: &Gen) -> (String, Ty, bool) {
        let fi = g.a.fns[key].clone();
        let adapter = format!("fang_ad_{}", fi.mangled);
        let mut params = Vec::new();
        for (i, p) in fi.params.iter().enumerate() {
            params.push(format!("{} p{}", g.ctype(&p.ty), i));
        }
        let arglist = (0..fi.params.len())
            .map(|i| format!("p{}", i))
            .collect::<Vec<_>>()
            .join(", ");
        let call = format!("{}({})", fi.mangled, arglist);
        let ret_c = g.ctype_ret(&fi.ret);
        if !g.stamped_sigs.borrow().contains(&adapter) {
            g.stamped_sigs.borrow_mut().insert(adapter.clone());
            if matches!(fi.ret, Ty::Void | Ty::Err) {
                writeln!(
                    g.extra_fns.borrow_mut(),
                    "static void {adapter}(void* env, {params}) {{\n    (void)env;\n    {call};\n}}",
                    adapter = adapter,
                    params = params.join(", "),
                    call = call,
                )
                .unwrap();
            } else {
                writeln!(
                    g.extra_fns.borrow_mut(),
                    "static {ret} {adapter}(void* env, {params}) {{\n    (void)env;\n    return {call};\n}}",
                    ret = ret_c,
                    adapter = adapter,
                    params = params.join(", "),
                    call = call,
                )
                .unwrap();
            }
        }
        let tmp = format!("__fnv{}", self.temp());
        self.w(&format!(
            "fang_closure* {tmp} = fang_closure_new(NULL, (void*){adapter}, NULL);",
            tmp = tmp,
            adapter = adapter,
        ));
        (tmp, Ty::Func(key.to_string()), true)
    }

    fn gen_expr(&mut self, e: &Expr, g: &Gen) -> (String, Ty, bool) {
        let ty = self
            .a
            .ty
            .get(&(self.module.clone(), e.span.start))
            .cloned()
            .unwrap_or(Ty::Err);
        match &e.kind {
            ExprKind::Int(v) => (v.to_string(), Ty::Int, false),
            ExprKind::Float(v) => {
                let s = crate::fmt::fmt_float(*v);
                let s = if s.contains('.') || s.contains('e') {
                    s
                } else {
                    format!("{}.0", s)
                };
                (s, Ty::Float, false)
            }
            ExprKind::Bool(v) => (v.to_string(), Ty::Bool, false),
            ExprKind::Nil => ("NULL".into(), Ty::Ptr, false),
            ExprKind::Str(s) => {
                let name = g.pool_str(s);
                (name, Ty::Str, false)
            }
            ExprKind::FStr(parts) => {
                let sb = format!("__sb{}", self.temp());
                self.w("fang_sb* {sb} = fang_sb_new();".replace("{sb}", &sb).as_str());
                for p in parts {
                    match p {
                        FStrPartAst::Lit(l) => {
                            let name = g.pool_str(l);
                            self.w(&format!("fang_sb_str({}, {});", sb, name));
                        }
                        FStrPartAst::E(sub) => {
                            let (code, t, fresh) = self.gen_expr(sub, g);
                            let _ = fresh;
                            match t {
                                Ty::Int => self.w(&format!("fang_sb_i64({}, {});", sb, code)),
                                Ty::Float => self.w(&format!("fang_sb_f64({}, {});", sb, code)),
                                Ty::Bool => self.w(&format!("fang_sb_bool({}, {});", sb, code)),
                                Ty::Str => self.w(&format!("fang_sb_str({}, {});", sb, code)),
                                Ty::Any => self.w(&format!("fang_sb_str({}, {});", sb, code)),
                                _ => {}
                            }
                        }
                    }
                }
                let out = format!("fang_sb_build({})", sb);
                (out, Ty::Str, true)
            }
            ExprKind::Ident(name) => {
                if let Some(member) = self.captures.get(name).cloned() {
                    let t = self.a.ty[&(self.module.clone(), e.span.start)].clone();
                    return (member, t, false);
                }
                if let Some(v) = self.lookup(name) {
                    return (format!("f.{}", v.field), v.ty.clone(), false);
                }
                // module-level global
                let gm = format!("g_{}_{}", self.module, name);
                if let Some(gt) = g.a.globals.get(&gm).cloned() {
                    return (gm, gt, false);
                }
                // Named function used as a value: wrap in a closure.
                let ty_here = self.a.ty[&(self.module.clone(), e.span.start)].clone();
                if let Ty::Func(key) = &ty_here {
                    if self.a.fns.contains_key(key) && !self.a.closures.contains_key(key) {
                        return self.gen_named_fn_value(key, g);
                    }
                }
                // constant
                let cm = format!("c_{}_{}", self.module, name);
                if let Some(cv) = g.a.consts.get(&cm) {
                    return self.const_val(cv, g);
                }
                // function reference
                ("NULL".into(), Ty::Err, false)
            }
            ExprKind::Unary(_, inner) => {
                let (code, t, _) = self.gen_expr(inner, g);
                (format!("(-{})", code), t, false)
            }
            ExprKind::Binary(op, l, r) => {
                let (lc, lt, _) = self.gen_expr(l, g);
                let (rc, rt, _) = self.gen_expr(r, g);
                self.gen_binary(*op, &lc, &lt, &rc, &rt, g)
            }
            ExprKind::And(l, r) => {
                let (lc, _, _) = self.gen_expr(l, g);
                let (rc, _, _) = self.gen_expr(r, g);
                (format!("(({}) && ({}))", lc, rc), Ty::Bool, false)
            }
            ExprKind::Or(l, r) => {
                let (lc, _, _) = self.gen_expr(l, g);
                let (rc, _, _) = self.gen_expr(r, g);
                (format!("(({}) || ({}))", lc, rc), Ty::Bool, false)
            }
            ExprKind::Not(inner) => {
                let (code, _, _) = self.gen_expr(inner, g);
                (format!("(!({}))", code), Ty::Bool, false)
            }
            ExprKind::Compare(op, l, r) => {
                let (lc, lt, _) = self.gen_expr(l, g);
                let (rc, rt, _) = self.gen_expr(r, g);
                self.gen_compare(*op, &lc, &lt, &rc, &rt, g)
            }
            ExprKind::Call(callee, args) => {
                self.gen_call(callee, args, e, g)
            }
            ExprKind::Method(recv, name, args) => {
                self.gen_method(recv, name, args, e, g)
            }
            ExprKind::Field(recv, name) => {
                // module constant
                if let ExprKind::Ident(base) = &recv.kind {
                    let gm2 = format!("g_{}_{}", base, name);
                    if let Some(t) = g.a.globals.get(&gm2).cloned() {
                        return (gm2, t, false);
                    }
                    let cm = format!("c_{}_{}", base, name);
                    if self.lookup(base).is_none() {
                        if let Some(cv) = g.a.consts.get(&cm) {
                            return self.const_val(cv, g);
                        }
                    }
                }
                let (rc, _, _) = self.gen_expr(recv, g);
                (format!("({}.{})", rc, name), ty, false)
            }
            ExprKind::Index(recv, idx) => {
                let (rc, rt, _) = self.gen_expr(recv, g);
                let (ic, _, _) = self.gen_expr(idx, g);
                match &rt {
                    Ty::List(elem) => (
                        format!("fang_list_{}_get({}, {})", g.sfx(elem), rc, ic),
                        (**elem).clone(),
                        false,
                    ),
                    Ty::Dict(k, v) if **k == Ty::Int => (
                        format!(
                            "fang_dict_{}_{}_get({}, {}, {})",
                            g.sfx(k),
                            g.sfx(v),
                            rc,
                            ic,
                            default_value(v)
                        ),
                        (**v).clone(),
                        false,
                    ),
                    Ty::Dict(k, v) => (
                        format!(
                            "fang_dict_{}_{}_get({}, {}, {})",
                            g.sfx(k),
                            g.sfx(v),
                            rc,
                            ic,
                            default_value(v)
                        ),
                        (**v).clone(),
                        false,
                    ),
                    Ty::Str => (
                        format!("fang_str_char_at({}, {})", rc, ic),
                        Ty::Str,
                        true,
                    ),
                    _ => ("NULL".into(), Ty::Err, false),
                }
            }
            ExprKind::Slice(recv, start, end) => {
                let (rc, rt, _) = self.gen_expr(recv, g);
                let sc = match start {
                    Some(s) => {
                        let (c, _, _) = self.gen_expr(s, g);
                        c
                    }
                    None => "0".into(),
                };
                let ec = match end {
                    Some(x) => {
                        let (c, _, _) = self.gen_expr(x, g);
                        c
                    }
                    None => format!("({})->len", rc)
                };
                match &rt {
                    Ty::List(elem) => (
                        format!("fang_list_{}_slice({}, {}, {})", g.sfx(elem), rc, sc, ec),
                        Ty::List(elem.clone()),
                        true,
                    ),
                    Ty::Str => (
                        format!("fang_str_slice({}, {}, {})", rc, sc, ec),
                        Ty::Str,
                        true,
                    ),
                    _ => ("NULL".into(), Ty::Err, false),
                }
            }
            ExprKind::ListLit(items) => {
                let elem = match &ty {
                    Ty::List(e) => (**e).clone(),
                    _ => Ty::Err,
                };
                let sfx = g.sfx(&elem);
                let lv = format!("__l{}", self.temp());
                self.w(&format!("fang_list_{}* {} = fang_list_{}_new();", sfx, lv, sfx));
                for item in items {
                    let (code, it, fresh) = self.gen_expr(item, g);
                    if g.is_rc(&it) && !fresh {
                        self.w(&format!("fang_incref((void*)({}));", code));
                    }
                    self.w(&format!("fang_list_{}_push({}, {});", sfx, lv, code));
                }
                (lv, Ty::List(Box::new(elem)), true)
            }
            ExprKind::DictLit(pairs) => {
                let (kt, vt) = match &ty {
                    Ty::Dict(k, v) => ((**k).clone(), (**v).clone()),
                    _ => (Ty::Err, Ty::Err),
                };
                let (ks, vs) = (g.sfx(&kt), g.sfx(&vt));
                let dv = format!("__d{}", self.temp());
                self.w(&format!(
                    "fang_dict_{}_{}* {} = fang_dict_{}_{}_new();",
                    ks, vs, dv, ks, vs
                ));
                for (k, v) in pairs {
                    let (kc, _, kfr) = self.gen_expr(k, g);
                    if !kfr {
                        self.w(&format!("fang_incref((void*)({}));", kc));
                    }
                    let (vc, _, vfr) = self.gen_expr(v, g);
                    if g.is_rc(&vt) && !vfr {
                        self.w(&format!("fang_incref((void*)({}));", vc));
                    }
                    self.w(&format!(
                        "fang_dict_{}_{}_set({}, {}, {});",
                        ks, vs, dv, kc, vc
                    ));
                }
                (dv, Ty::Dict(Box::new(kt), Box::new(vt)), true)
            }
            ExprKind::TupleLit(a, b) => {
                let (ac, at, af) = self.gen_expr(a, g);
                let (bc, bt, bf) = self.gen_expr(b, g);
                let (sfxa, sfxb) = (g.sfx(&at), g.sfx(&bt));
                if g.is_rc(&at) && !af {
                    self.w(&format!("fang_incref((void*)({}));", ac));
                }
                if g.is_rc(&bt) && !bf {
                    self.w(&format!("fang_incref((void*)({}));", bc));
                }
                (
                    format!("((fang_tuple_{}_{}) {{ {}, {} }})", sfxa, sfxb, ac, bc),
                    Ty::Tuple(Box::new(at), Box::new(bt)),
                    true,
                )
            }
            ExprKind::FnExpr(f) => {
                let key = match &ty {
                    Ty::Func(k) => k.clone(),
                    _ => "lam_unknown".to_string(),
                };
                let cinfo = g.a.closures.get(&key).cloned();
                let Some(cinfo) = cinfo else {
                    return ("NULL".into(), Ty::Err, false);
                };
                // Stamp the env struct, its constructor/destructor, and the
                // body function; then create the closure value here.
                let env_ty = format!("fang_env_{}", key);
                let mut env_fields = String::new();
                let mut cap_members: HashMap<String, String> = HashMap::new();
                let _cap_args: Vec<String> = Vec::new();
                let mut cap_tys = Vec::new();
                for (i, (cname, cty)) in cinfo.captures.iter().enumerate() {
                    let field = format!("cap_{}", i);
                    env_fields.push_str(&format!("    {} {};\n", g.ctype(cty), field));
                    cap_members.insert(cname.clone(), format!("env->{}", field));
                    cap_tys.push(cty.clone());
                }
                let mut env_new_params = Vec::new();
                for (i, ct) in cap_tys.iter().enumerate() {
                    env_new_params.push(format!("{} c{}", g.ctype(ct), i));
                }
                let mut env_new_increfs = String::new();
                for (i, ct) in cap_tys.iter().enumerate() {
                    if g.is_rc(ct) {
                        env_new_increfs
                            .push_str(&format!("    fang_incref((void*)(e->cap_{}));\n", i));
                    }
                }
                let mut env_dec = String::new();
                for (i, ct) in cap_tys.iter().enumerate() {
                    if g.is_rc(ct) {
                        env_dec.push_str(&format!(
                            "    {}\n",
                            g.decref_stmt(&format!("e->cap_{}", i), ct)
                        ));
                    }
                }
                writeln!(
                    g.extra_decls.borrow_mut(),
                    "typedef struct {{\n{}}} {env};\nstatic {env}* {env}_new({params}) {{\n    {env}* e = ({env}*)malloc(sizeof({env}));\n    if (!e) fang_panic(\"out of memory\", \"<runtime>\", 0);\n{assign}{increfs}    return e;\n}}\nstatic void {env}_destroy(void* ve) {{\n    {env}* e = ({env}*)ve;\n{dec}    free(e);\n}}",
                    env_fields,
                    env = env_ty,
                    params = if env_new_params.is_empty() { "void".to_string() } else { env_new_params.join(", ") },
                    assign = (0..cap_tys.len())
                        .map(|i| format!("    e->cap_{} = c{};\n", i, i))
                        .collect::<String>(),
                    increfs = env_new_increfs,
                    dec = env_dec,
                )
                .unwrap();

                // Body function: locals frame + captures as env members.
                let ret_c = g.ctype_ret(&cinfo.ret);
                let mut param_names = Vec::new();
                let mut fg = FnGen {
                    module: self.module.clone(),
                    path: self.path.clone(),
                    a: g.a,
                    captures: cap_members,
                    buf: String::new(),
                    indent: 1,
                    scopes: vec![HashMap::new()],
                    locals: Vec::new(),
                    has_temps: Vec::new(),
                    frame: String::new(),
                    field_n: 0,
                    temp_n: 0,
                    ret: cinfo.ret.clone(),
                };
                let mut proto_params = vec![format!("{}* env", env_ty)];
                for (i, pt) in cinfo.params.iter().enumerate() {
                    let pname = f.params.get(i).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", i));
                    proto_params.push(format!("{} p{}", g.ctype(pt), i));
                    param_names.push(pname);
                }
                for (i, pt) in cinfo.params.iter().enumerate() {
                    let field = format!("{}__{}", param_names[i], fg.next_field());
                    let inc = if g.is_rc(pt) {
                        format!("    fang_incref((void*)(p{}));\n", i)
                    } else {
                        String::new()
                    };
                    fg.frame.push_str(&format!("    {} {};\n", g.ctype(pt), field));
                    fg.buf.push_str(&format!("    f.{} = p{};\n{}", field, i, inc));
                    let var = Var { field: field.clone(), ty: pt.clone() };
                    fg.scopes.last_mut().unwrap().insert(param_names[i].clone(), var.clone());
                    fg.locals.push(var);
                }
                emit_body_with_trailing(&mut fg, &f.body, &cinfo.ret, g);
                let cleanup = fg.cleanup_stmts(g);
                fg.buf.push_str(&cleanup);
                let is_void = matches!(cinfo.ret, Ty::Void | Ty::Err);
                let ret_decl = if is_void { String::new() } else {
                    format!("    {} __ret;\n", ret_c)
                };
                let init = if fg.frame.trim().is_empty() { String::new() } else { " = {0}".to_string() };
                writeln!(
                    g.extra_fns.borrow_mut(),
                    "static {ret} fang_{key}({params}) {{\n{ret_decl}    struct {{\n{frame}    }} f{init};\n{body}__exit: ;\n{ret_stmt}}}",
                    ret = ret_c,
                    key = key,
                    params = proto_params.join(", "),
                    ret_decl = ret_decl,
                    frame = fg.frame,
                    init = init,
                    body = fg.buf,
                    ret_stmt = if is_void { "    return;\n".to_string() } else { "    return __ret;\n".to_string() },
                )
                .unwrap();

                // Create the closure at this site.
                let tmp = format!("__lam{}", self.temp());
                let mut cap_exprs = Vec::new();
                for (cname, _) in &cinfo.captures {
                    if let Some(v) = self.lookup(cname) {
                        cap_exprs.push(format!("f.{}", v.field));
                    } else {
                        cap_exprs.push("NULL".into());
                    }
                }
                self.w(&format!(
                    "fang_closure* {tmp} = fang_closure_new({env}_new({caps}), (void*)fang_{key}, {env}_destroy);",
                    tmp = tmp,
                    env = env_ty,
                    caps = cap_exprs.join(", "),
                    key = key,
                ));
                (tmp, Ty::Func(key), true)
            }
            ExprKind::Comprehension { item, binders, iter, cond } => {
                let elem = match &ty {
                    Ty::List(t) => (**t).clone(),
                    _ => Ty::Err,
                };
                let sfx = g.sfx(&elem);
                // Range iters loop on a counter; collection iters walk
                // a materialized list (dicts iterate their keys).
                let is_range = matches!(&**iter, Iter::Range { .. });
                let (it_var, loop_head, needs_release) = if is_range {
                    let (s, start_expr, _, _) = match &**iter {
                        Iter::Range { start, .. } => {
                            let se = self.gen_expr(start, g);
                            (start, se, Ty::Int, ())
                        }
                        _ => unreachable!(),
                    };
                    let _ = s;
                    let (start_code, _, _) = (start_expr.0.clone(), start_expr.1.clone(), start_expr.2);
                    let (end_code, _, _) = match &**iter {
                        Iter::Range { end, .. } => self.gen_expr(end, g),
                        _ => unreachable!(),
                    };
                    let inclusive = matches!(&**iter, Iter::Range { inclusive: true, .. });
                    let var = format!("__ci{}", self.temp());
                    (
                        var.clone(),
                        format!(
                            "for (int64_t {} = {}, __ce = {}; {} {} __ce; {}++) {{",
                            var,
                            start_code,
                            end_code,
                            var,
                            if inclusive { "<=" } else { "<" },
                            var
                        ),
                        false,
                    )
                } else {
                    let it = self.a.ty[&(self.module.clone(), e.span.start)].clone();
                    let _ = it;
                    let it_ty = match &**iter {
                        Iter::Expr(ie) => self.a.ty[&(self.module.clone(), ie.span.start)].clone(),
                        _ => Ty::Err,
                    };
                    let (code, _, fresh) = match &**iter {
                        Iter::Expr(ie) => self.gen_expr(ie, g),
                        _ => ("0".into(), Ty::Int, false),
                    };
                    let list_ty = match &it_ty {
                        Ty::Dict(k, _) => Ty::List(k.clone()),
                        other => other.clone(),
                    };
                    let list_expr = match &it_ty {
                        Ty::Dict(k, v) => format!(
                            "fang_dict_{}_{}_keys({})",
                            g.sfx(k),
                            g.sfx(v),
                            code
                        ),
                        _ => code.clone(),
                    };
                    let var = format!("__cit{}", self.temp());
                    self.w(&format!(
                        "{} {} = {};",
                        g.ctype(&list_ty),
                        var,
                        list_expr
                    ));
                    let release = (fresh || matches!(it_ty, Ty::Dict(_, _))) && g.is_rc(&list_ty);
                    (
                        var.clone(),
                        format!("for (int64_t __cj = 0; __cj < {}->len; __cj++) {{", var),
                        release,
                    )
                };
                let out = format!("__cmp{}", self.temp());
                self.w(&format!(
                    "fang_list_{}* {} = fang_list_{}_new();",
                    sfx, out, sfx
                ));
                // binder types from sema
                let binder_tys = g.a
                    .for_binders
                    .get(&(self.module.clone(), e.span.start))
                    .cloned()
                    .unwrap_or_default();
                self.scopes.push(HashMap::new());
                let mut binder_fields: Vec<(String, Ty)> = Vec::new();
                for (bi, b) in binders.iter().enumerate() {
                    let bt = binder_tys.get(bi).cloned().unwrap_or(Ty::Err);
                    let bf = format!("{}__{}", b, self.next_field());
                    self.frame.push_str(&format!("    {} {};\n", g.ctype(&bt), bf));
                    self.scopes.last_mut().unwrap().insert(
                        b.clone(),
                        Var { field: bf.clone(), ty: bt.clone() },
                    );
                    self.locals.push(Var { field: bf.clone(), ty: bt.clone() });
                    binder_fields.push((bf, bt));
                }
                self.w(&loop_head);
                self.indent += 1;
                if is_range {
                    self.w(&format!("f.{} = {};", binder_fields[0].0, it_var));
                } else if binder_fields.len() == 2 {
                    let cpn = self.temp();
                    let cp = format!("__cp{}", cpn);
                    self.w(&format!(
                        "fang_tuple_{}_{} {} = {}->elems[__cj];",
                        g.sfx(&binder_fields[0].1),
                        g.sfx(&binder_fields[1].1),
                        cp,
                        it_var
                    ));
                    self.w(&format!("f.{} = {}.a;", binder_fields[0].0, cp));
                    self.w(&format!("f.{} = {}.b;", binder_fields[1].0, cp));
                } else {
                    self.w(&format!(
                        "f.{} = {}->elems[__cj];",
                        binder_fields[0].0, it_var
                    ));
                }
                for (bf, bt) in &binder_fields {
                    match bt {
                        t if g.is_rc(t) => {
                            self.w(&format!("fang_incref((void*)(f.{}));", bf));
                        }
                        Ty::Struct(m) => {
                            for si in g.struct_field_incref(&format!("f.{}", bf), m) {
                                self.w(&si);
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(c) = cond {
                    let (cc, _, _) = self.gen_expr(c, g);
                    self.w(&format!("if ({}) {{", cc));
                    self.indent += 1;
                }
                let (icode, it2, ifr) = self.gen_expr(item, g);
                if g.is_rc(&it2) && !ifr {
                    self.w(&format!("fang_incref((void*)({}));", icode));
                }
                self.w(&format!("fang_list_{}_push({}, {});", sfx, out, icode));
                if cond.is_some() {
                    self.indent -= 1;
                    self.w("}");
                }
                // per-iteration binder cleanup
                for (bf, bt) in &binder_fields {
                    match bt {
                        t if g.is_rc(t) => {
                            let d = g.decref_stmt(&format!("f.{}", bf), t);
                            if !d.is_empty() {
                                self.w(&d);
                            }
                        }
                        Ty::Struct(m) => {
                            for sd in g.struct_field_decref(&format!("f.{}", bf), m) {
                                self.w(&sd);
                            }
                        }
                        _ => {}
                    }
                }
                self.indent -= 1;
                self.w("}");
                if needs_release {
                    let d = g.decref_stmt(&it_var, &Ty::List(Box::new(Ty::Err)));
                    let _ = d;
                }
                self.scopes.pop();
                (out, ty.clone(), true)
            }
            ExprKind::Cast(t, inner) => {
                let (code, it, _) = self.gen_expr(inner, g);
                match (t, &it) {
                    (Type::Ptr, Ty::Int) => (
                        format!("((void*)(intptr_t)({}))", code),
                        Ty::Ptr,
                        false,
                    ),
                    (Type::Int, Ty::Ptr) => (
                        format!("((int64_t)(uintptr_t)({}))", code),
                        Ty::Int,
                        false,
                    ),
                    (Type::Ptr, Ty::Str | Ty::List(_) | Ty::Dict(_, _)) => {
                        // Storing an RC value behind a raw pointer: take a
                        // keep-alive reference. Raw pointer fields are never
                        // released, so the value lives for the process
                        // lifetime (documented model for ptr-cast holders).
                        self.w(&format!("fang_incref((void*)({}));", code));
                        (
                            format!("((void*)({}))", code),
                            Ty::Ptr,
                            false,
                        )
                    }
                    (Type::Str, Ty::Ptr) if matches!(t, Type::Str) => (
                        format!("((fang_str*)({}))", code),
                        Ty::Str,
                        false,
                    ),
                    (Type::List(_), Ty::Ptr) | (Type::Dict(_, _), Ty::Ptr) => {
                        let target =
                            self.a.ty[&(self.module.clone(), e.span.start)].clone();
                        let cty = g.ctype(&target);
                        (
                            format!("(({})({}))", cty, code),
                            target,
                            false,
                        )
                    }
                    _ => (code, it.clone(), false),
                }
            }
        }
    }

    fn const_val(&mut self, cv: &ConstVal, g: &Gen) -> (String, Ty, bool) {
        match cv {
            ConstVal::Int(v) => (v.to_string(), Ty::Int, false),
            ConstVal::Float(v) => (crate::fmt::fmt_float(*v), Ty::Float, false),
            ConstVal::Bool(v) => (v.to_string(), Ty::Bool, false),
            ConstVal::Str(s) => {
                let name = g.pool_str(s);
                (name, Ty::Str, false)
            }
        }
    }

    fn gen_binary(
        &mut self,
        op: BinOp,
        lc: &str,
        lt: &Ty,
        rc: &str,
        rt: &Ty,
        g: &Gen,
    ) -> (String, Ty, bool) {
        use BinOp::*;
        // str + str
        if *lt == Ty::Str && *rt == Ty::Str && op == Add {
            return (
                format!("fang_str_concat({}, {})", lc, rc),
                Ty::Str,
                true,
            );
        }
        // list + list
        if let (Ty::List(a), Ty::List(b)) = (lt, rt) {
            if op == Add && a == b {
                return (
                    format!(
                        "fang_list_{}_concat({}, {})",
                        g.sfx(a),
                        lc,
                        rc
                    ),
                    Ty::List(a.clone()),
                    true,
                );
            }
        }
        // numeric with promotion
        let promoted = if lt == &Ty::Float || rt == &Ty::Float {
            Ty::Float
        } else {
            Ty::Int
        };
        let (lc2, rc2) = promote(lc, lt, &promoted, rc, rt);
        let c = match op {
            Add => format!("(({}) + ({}))", lc2, rc2),
            Sub => format!("(({}) - ({}))", lc2, rc2),
            Mul => format!("(({}) * ({}))", lc2, rc2),
            Div => format!("(({}) / ({}))", lc2, rc2),
            Mod => format!("(({}) % ({}))", lc2, rc2),
            BitAnd => format!("(({}) & ({}))", lc2, rc2),
            BitOr => format!("(({}) | ({}))", lc2, rc2),
            BitXor => format!("(({}) ^ ({}))", lc2, rc2),
            Shl => format!("(({}) << ({}))", lc2, rc2),
            Shr => format!("(({}) >> ({}))", lc2, rc2),
            Pow => {
                if promoted == Ty::Float {
                    format!("pow({}, {})", lc2, rc2)
                } else {
                    format!("((int64_t)pow((double){}, (double){}))", lc2, rc2)
                }
            }
        };
        let needs_math = matches!(op, Pow);
        if needs_math {
            // math.h is pulled in by the runtime; nothing to do
        }
        (c, promoted, false)
    }

    fn gen_compare(
        &mut self,
        op: CmpOp,
        lc: &str,
        lt: &Ty,
        rc: &str,
        rt: &Ty,
        g: &Gen,
    ) -> (String, Ty, bool) {
        use CmpOp::*;
        let _ = g;
        if *lt == Ty::Str && *rt == Ty::Str {
            let c = match op {
                Eq => format!("fang_str_eq({}, {})", lc, rc),
                Ne => format!("(!fang_str_eq({}, {}))", lc, rc),
                Lt => format!("(fang_str_cmp({}, {}) < 0)", lc, rc),
                Gt => format!("(fang_str_cmp({}, {}) > 0)", lc, rc),
                Le => format!("(fang_str_cmp({}, {}) <= 0)", lc, rc),
                Ge => format!("(fang_str_cmp({}, {}) >= 0)", lc, rc),
            };
            return (c, Ty::Bool, false);
        }
        if let Ty::Struct(m) = lt {
            let c = match op {
                Eq => format!("fang_{}_eq({}, {})", m, lc, rc),
                Ne => format!("(!fang_{}_eq({}, {}))", m, lc, rc),
                _ => "false".into(),
            };
            return (c, Ty::Bool, false);
        }
        let promoted = if lt == &Ty::Float || rt == &Ty::Float {
            Ty::Float
        } else {
            Ty::Int
        };
        let (lc2, rc2) = promote(lc, lt, &promoted, rc, rt);
        let sym = match op {
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Gt => ">",
            Le => "<=",
            Ge => ">=",
        };
        (
            format!("(({}) {} ({}))", lc2, sym, rc2),
            Ty::Bool,
            false,
        )
    }

    fn gen_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        call_expr: &Expr,
        g: &Gen,
    ) -> (String, Ty, bool) {
        let target = self
            .a
            .call
            .get(&(self.module.clone(), call_expr.span.start))
            .cloned();
        let ret_ty = self
            .a
            .ty
            .get(&(self.module.clone(), call_expr.span.start))
            .cloned()
            .unwrap_or(Ty::Err);
        let ret_ty = ret_ty.clone();

        let skip_first = matches!(&target, Some(CallTarget::FnSelf(_)));
        match target {
            Some(CallTarget::FnArg) => {
                for arg in args {
                    let _ = self.gen_expr(arg, g);
                }
                ("NULL".into(), Ty::Err, false)
            }
            Some(CallTarget::Fn(mangled)) | Some(CallTarget::FnSelf(mangled)) => {
                let fi = g.a.fns[&mangled].clone();
                let call_name = if fi.is_extern { fi.c_name.clone() } else { mangled.clone() };
                let mut arg_codes = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let pi = if skip_first { i + 1 } else { i };
                    let pt = fi.params.get(pi).map(|p| p.ty.clone()).unwrap_or(Ty::Err);
                    let (code, at, fresh) = self.gen_expr(arg, g);
                    if g.is_rc(&pt) && !fresh {
                        self.w(&format!("fang_incref((void*)({}));", code));
                    }
                    let _ = at;
                    arg_codes.push(code);
                }
                // Omitted trailing parameters fill from their defaults,
                // evaluated here at the call site.
                let first_missing = if skip_first { args.len() + 1 } else { args.len() };
                for pi in first_missing..fi.params.len() {
                    if let Some(Some(def)) = fi.defaults.get(pi).map(|d| d.as_ref()) {
                        let pt = fi.params[pi].ty.clone();
                        let saved_module = self.module.clone();
                        self.module = fi.module.clone();
                        let (code, _, fr) = self.gen_expr(def, g);
                        self.module = saved_module;
                        if g.is_rc(&pt) && !fr {
                            self.w(&format!("fang_incref((void*)({}));", code));
                        }
                        arg_codes.push(code);
                    }
                }
                let fresh = g.is_rc(&ret_ty);
                (
                    format!("{}({})", call_name, arg_codes.join(", ")),
                    ret_ty,
                    fresh,
                )
            }
            Some(CallTarget::StructNew(mangled)) => {
                let si = g.a.structs[&mangled].clone();
                let mut arg_codes = Vec::new();
                for (arg, (_, ft)) in args.iter().zip(si.fields.iter()) {
                    let (code, _, fresh) = self.gen_expr(arg, g);
                    if g.is_rc(ft) && !fresh {
                        self.w(&format!("fang_incref((void*)({}));", code));
                    }
                    arg_codes.push(code);
                }
                (
                    format!("fang_{}_new({})", mangled, arg_codes.join(", ")),
                    Ty::Struct(mangled),
                    false,
                )
            }
            Some(CallTarget::BuiltinPrint) => {
                for (i, arg) in args.iter().enumerate() {
                    let (code, t, _) = self.gen_expr(arg, g);
                    let call = match t {
                        Ty::Int => format!("fang_print_i64({})", code),
                        Ty::Float => format!("fang_print_f64({})", code),
                        Ty::Bool => format!("fang_print_bool({})", code),
                        Ty::Str | Ty::Any => format!("fang_print_str({})", code),
                        Ty::Ptr => format!("fang_print_ptr({})", code),
                        Ty::List(e) => format!(
                            "fang_print_debug((void*)({}), {})",
                            code,
                            debug_kind(&e)
                        ),
                        Ty::Tuple(..) => format!(
                            "fang_print_debug((void*)(&({})), 5)",
                            code
                        ),
                        Ty::Dict(_, _) => format!(
                            "fang_print_debug((void*)({}), 6)",
                            code
                        ),
                        _ => "fang_print_i64(0)".into(),
                    };
                    self.w(&format!("{};", call));
                    if i + 1 < args.len() {
                        self.w("fang_print_sep();");
                    }
                }
                self.w("fang_print_nl();");
                ("0".into(), Ty::Void, false)
            }
            Some(CallTarget::BuiltinLen) => {
                let (code, t, _) = self.gen_expr(&args[0], g);
                let c = match &t {
                    Ty::Str => format!("({})->len", code),
                    Ty::List(_) => format!("({})->len", code),
                    Ty::Dict(_, _) => format!("({})->len", code),
                    _ => "0".into(),
                };
                (c, Ty::Int, false)
            }
            Some(CallTarget::BuiltinInput) => {
                let prompt = if args.is_empty() {
                    "NULL".to_string()
                } else {
                    let (pc, _, _) = self.gen_expr(&args[0], g);
                    pc
                };
                (format!("fang_input_prompt({})", prompt), Ty::Str, true)
            }
            Some(CallTarget::BuiltinStr) => {
                let (code, t, _) = self.gen_expr(&args[0], g);
                let c = match t {
                    Ty::Int => format!("fang_str_from_i64({})", code),
                    Ty::Float => format!("fang_str_from_f64({})", code),
                    Ty::Bool => format!("fang_str_from_bool({})", code),
                    Ty::Str => code,
                    Ty::List(e) => format!(
                        "fang_debug_str((void*)({}), {})",
                        code,
                        debug_kind(&e)
                    ),
                    Ty::Dict(_, _) => format!("fang_debug_str((void*)({}), 6)", code),
                    _ => "fang_str_from_i64(0)".to_string(),
                };
                (c, Ty::Str, true)
            }
            Some(CallTarget::BuiltinAssert) => {
                let (code, _, _) = self.gen_expr(&args[0], g);
                self.w(&format!(
                    "if (!({})) fang_panic(\"assertion failed\", __FILE__, __LINE__);",
                    code
                ));
                ("0".into(), Ty::Void, false)
            }
            Some(CallTarget::ListMethod(op)) => {
                let (recv_code, rt, _) = self.gen_expr(callee_as_recv(callee), g);
                let elem = match &rt {
                    Ty::List(e) => (**e).clone(),
                    _ => Ty::Err,
                };
                let sfx = g.sfx(&elem);
                match op {
                    ListOp::Push => {
                        let (code, at, fresh) = self.gen_expr(&args[0], g);
                        if g.is_rc(&at) && !fresh {
                            self.w(&format!("fang_incref((void*)({}));", code));
                        }
                        self.w(&format!(
                            "fang_list_{}_push({}, {});",
                            sfx, recv_code, code
                        ));
                        ("0".into(), Ty::Void, false)
                    }
                    ListOp::Pop => (
                        format!("fang_list_{}_pop({})", sfx, recv_code),
                        elem,
                        true,
                    ),
                    ListOp::Insert => {
                        let (ic, _, _) = self.gen_expr(&args[0], g);
                        let (vc, at, fresh) = self.gen_expr(&args[1], g);
                        if g.is_rc(&at) && !fresh {
                            self.w(&format!("fang_incref((void*)({}));", vc));
                        }
                        self.w(&format!(
                            "fang_list_{}_insert({}, {}, {});",
                            sfx, recv_code, ic, vc
                        ));
                        ("0".into(), Ty::Void, false)
                    }
                    ListOp::RemoveAt => {
                        let (ic, _, _) = self.gen_expr(&args[0], g);
                        self.w(&format!(
                            "fang_list_{}_remove_at({}, {});",
                            sfx, recv_code, ic
                        ));
                        ("0".into(), Ty::Void, false)
                    }
                    ListOp::Map => {
                        let arg0 = args.first().cloned();
                        let Some(fn_arg) = arg0 else {
                            return ("NULL".into(), Ty::Err, false);
                        };
                        let fkey = match self.a.ty[&(self.module.clone(), fn_arg.span.start)].clone() {
                            Ty::Func(k) => k,
                            _ => return ("NULL".into(), Ty::Err, false),
                        };
                        let (params, fret) = self.sig_of(&fkey, g);
                        let sig = g.stamp_trampoline(&fkey, &params, &fret);
                        let out_elem = fret.clone();
                        let osfx = g.sfx(&out_elem);
                        let (fcode, _, _) = self.gen_expr(&fn_arg, g);
                        let out = format!("__map{}", self.temp());
                        self.w(&format!(
                            "fang_list_{}* {} = fang_list_{}_new();",
                            osfx, out, osfx
                        ));
                        self.w(&format!(
                            "for (int64_t __mj = 0; __mj < {}->len; __mj++) {{",
                            recv_code
                        ));
                        self.indent += 1;
                        self.w(&format!(
                            "fang_list_{}_push({}, {}({}, {}->elems[__mj]));",
                            osfx, out, sig, fcode, recv_code
                        ));
                        self.indent -= 1;
                        self.w("}");
                        (out, Ty::List(Box::new(out_elem)), true)
                    }
                    ListOp::Filter => {
                        let arg0 = args.first().cloned();
                        let Some(fn_arg) = arg0 else {
                            return ("NULL".into(), Ty::Err, false);
                        };
                        let fkey = match self.a.ty[&(self.module.clone(), fn_arg.span.start)].clone() {
                            Ty::Func(k) => k,
                            _ => return ("NULL".into(), Ty::Err, false),
                        };
                        let (params, fret) = self.sig_of(&fkey, g);
                        let sig = g.stamp_trampoline(&fkey, &params, &fret);
                        let (fcode, _, _) = self.gen_expr(&fn_arg, g);
                        let out = format!("__flt{}", self.temp());
                        let osfx = g.sfx(&elem);
                        self.w(&format!(
                            "fang_list_{}* {} = fang_list_{}_new();",
                            osfx, out, osfx
                        ));
                        self.w(&format!(
                            "for (int64_t __fj = 0; __fj < {}->len; __fj++) {{",
                            recv_code
                        ));
                        self.indent += 1;
                        self.w(&format!(
                            "if ({}({}, {}->elems[__fj])) {{",
                            sig, fcode, recv_code
                        ));
                        self.indent += 1;
                        self.w(&format!(
                            "fang_list_{}_push({}, {}->elems[__fj]);",
                            osfx, out, recv_code
                        ));
                        self.indent -= 1;
                        self.w("}");
                        self.indent -= 1;
                        self.w("}");
                        (out, Ty::List(Box::new(elem.clone())), true)
                    }
                    ListOp::Reduce => {
                        let arg0 = args.first().cloned();
                        let init = args.get(1).cloned();
                        let (Some(fn_arg), Some(init_expr)) = (arg0, init) else {
                            return ("NULL".into(), Ty::Err, false);
                        };
                        let fkey = match self.a.ty[&(self.module.clone(), fn_arg.span.start)].clone() {
                            Ty::Func(k) => k,
                            _ => return ("NULL".into(), Ty::Err, false),
                        };
                        let (params, fret) = self.sig_of(&fkey, g);
                        let sig = g.stamp_trampoline(&fkey, &params, &fret);
                        let (fcode, _, _) = self.gen_expr(&fn_arg, g);
                        let (icode, it, ifr) = self.gen_expr(&init_expr, g);
                        if g.is_rc(&it) && !ifr {
                            self.w(&format!("fang_incref((void*)({}));", icode));
                        }
                        let acc = format!("__acc{}", self.temp());
                        self.w(&format!("{} {} = {};", g.ctype(&it), acc, icode));
                        self.w(&format!(
                            "for (int64_t __rj = 0; __rj < {}->len; __rj++) {{",
                            recv_code
                        ));
                        self.indent += 1;
                        self.w(&format!(
                            "{} = {}({}, {}, {}->elems[__rj]);",
                            acc, sig, fcode, acc, recv_code
                        ));
                        self.indent -= 1;
                        self.w("}");
                        (acc, it, true)
                    }
                    ListOp::Sort => {
                        match &elem {
                            Ty::Int => self.w(&format!(
                                "fang_sort_i64({}->elems, {}->len);",
                                recv_code, recv_code
                            )),
                            Ty::Float => self.w(&format!(
                                "fang_sort_f64({}->elems, {}->len);",
                                recv_code, recv_code
                            )),
                            Ty::Str => self.w(&format!(
                                "fang_sort_str({}->elems, {}->len);",
                                recv_code, recv_code
                            )),
                            _ => {}
                        }
                        ("0".into(), Ty::Void, false)
                    }
                    ListOp::Reverse => {
                        self.w(&format!(
                            "fang_list_{}_reverse({});",
                            sfx, recv_code
                        ));
                        ("0".into(), Ty::Void, false)
                    }
                    ListOp::Contains => (
                        format!("fang_list_{}_contains({})", sfx, recv_code),
                        Ty::Bool,
                        false,
                    ),
                    ListOp::IndexOf => (
                        format!("fang_list_{}_index_of({})", sfx, recv_code),
                        Ty::Int,
                        false,
                    ),
                }
            }
            Some(CallTarget::DictMethod(op)) => {
                let (recv_code, rt, _) = self.gen_expr(callee_as_recv(callee), g);
                let (kt, vt) = match &rt {
                    Ty::Dict(k, v) => ((**k).clone(), (**v).clone()),
                    _ => (Ty::Err, Ty::Err),
                };
                let (ks, vs) = (g.sfx(&kt), g.sfx(&vt));
                match op {
                    DictOp::Get => {
                        let (kc, _, kfr) = self.gen_expr(&args[0], g);
                        if g.is_rc(&kt) && !kfr {
                            self.w(&format!("fang_incref((void*)({}));", kc));
                        }
                        let (dc, _, dfr) = self.gen_expr(&args[1], g);
                        if g.is_rc(&vt) && !dfr {
                            self.w(&format!("fang_incref((void*)({}));", dc));
                        }
                        (
                            format!(
                                "fang_dict_{}_{}_get({}, {}, {})",
                                ks, vs, recv_code, kc, dc
                            ),
                            vt,
                            false,
                        )
                    }
                    DictOp::Set => {
                        let (kc, _, kfr) = self.gen_expr(&args[0], g);
                        if g.is_rc(&kt) && !kfr {
                            self.w(&format!("fang_incref((void*)({}));", kc));
                        }
                        let (vc, _, vfr) = self.gen_expr(&args[1], g);
                        if g.is_rc(&vt) && !vfr {
                            self.w(&format!("fang_incref((void*)({}));", vc));
                        }
                        self.w(&format!(
                            "fang_dict_{}_{}_set({}, {}, {});",
                            ks, vs, recv_code, kc, vc
                        ));
                        ("0".into(), Ty::Void, false)
                    }
                    DictOp::Keys => (
                        format!("fang_dict_{}_{}_keys({})", ks, vs, recv_code),
                        Ty::List(Box::new(kt)),
                        true,
                    ),
                    DictOp::Values => (
                        format!("fang_dict_{}_{}_values({})", ks, vs, recv_code),
                        Ty::List(Box::new(vt)),
                        true,
                    ),
                    DictOp::Items => (
                        format!("fang_dict_{}_{}_items({})", ks, vs, recv_code),
                        Ty::List(Box::new(Ty::Tuple(Box::new(kt), Box::new(vt)))),
                        true,
                    ),
                    DictOp::Contains => {
                        let (kc, _, kfr) = self.gen_expr(&args[0], g);
                        if g.is_rc(&kt) && !kfr {
                            self.w(&format!("fang_incref((void*)({}));", kc));
                        }
                        (
                            format!(
                                "fang_dict_{}_{}_contains({}, {})",
                                ks, vs, recv_code, kc
                            ),
                            Ty::Bool,
                            false,
                        )
                    }
                    DictOp::Reserve => {
                        let (nc, _, _) = self.gen_expr(&args[0], g);
                        self.w(&format!(
                            "fang_dict_{}_{}_reserve({}, {});",
                            ks, vs, recv_code, nc
                        ));
                        ("0".into(), Ty::Void, false)
                    }
                    DictOp::Remove => {
                        let (kc, _, kfr) = self.gen_expr(&args[0], g);
                        if g.is_rc(&kt) && !kfr {
                            self.w(&format!("fang_incref((void*)({}));", kc));
                        }
                        self.w(&format!(
                            "fang_dict_{}_{}_remove({}, {});",
                            ks, vs, recv_code, kc
                        ));
                        ("0".into(), Ty::Void, false)
                    }
                }
            }
            Some(CallTarget::BuiltinFloat) => {
                let (code, _, _) = self.gen_expr(&args[0], g);
                (format!("((double)({}))", code), Ty::Float, false)
            }
            Some(CallTarget::BuiltinInt) => {
                let (code, _, _) = self.gen_expr(&args[0], g);
                (format!("((int64_t)({}))", code), Ty::Int, false)
            }
            Some(CallTarget::ClosureCall(key)) => {
                // Resolve the signature, stamp a trampoline, call through it.
                let (params, ret) = {
                    if let Some(ci) = g.a.closures.get(&key) {
                        (ci.params.clone(), ci.ret.clone())
                    } else if let Some(fi) = g.a.fns.get(&key) {
                        (fi.params.iter().map(|p| p.ty.clone()).collect(), fi.ret.clone())
                    } else {
                        (Vec::new(), Ty::Err)
                    }
                };
                let sig = g.stamp_trampoline(&key, &params, &ret);
                let mut arg_codes = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let pt = params.get(i).cloned().unwrap_or(Ty::Err);
                    let (code, at, fr) = self.gen_expr(arg, g);
                    if g.is_rc(&pt) && !fr && pt != Ty::Err {
                        // borrowed argument into an owned parameter
                        match pt {
                            Ty::Func(_) => {}
                            _ => self.w(&format!("fang_incref((void*)({}));", code)),
                        }
                    }
                    let _ = (at, fr);
                    arg_codes.push(code);
                }
                // The callee expression: the closure value itself.
                let (ccode, _, _) = self.gen_expr_borrowed(callee, g);
                let fresh = g.is_rc(&ret);
                let call = if arg_codes.is_empty() {
                    format!("{}({})", sig, ccode)
                } else {
                    format!("{}({}, {})", sig, ccode, arg_codes.join(", "))
                };
                (call, ret, fresh)
            }
            Some(CallTarget::SigCall(params, ret)) => {
                let key = format!(
                    "sig_{}_{}",
                    params.iter().map(|p| g.sfx(p)).collect::<Vec<_>>().join("_"),
                    g.sfx(&ret)
                );
                let sig = g.stamp_trampoline(&key, &params, &ret);
                let mut arg_codes = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let pt = params.get(i).cloned().unwrap_or(Ty::Err);
                    let (code, _, fr) = self.gen_expr(arg, g);
                    if g.is_rc(&pt) && !fr && !matches!(pt, Ty::Sig(..) | Ty::Func(_)) {
                        self.w(&format!("fang_incref((void*)({}));", code));
                    }
                    arg_codes.push(code);
                }
                let (ccode, _, _) = self.gen_expr_borrowed(callee, g);
                let fresh = g.is_rc(&ret);
                let call = if arg_codes.is_empty() {
                    format!("{}({})", sig, ccode)
                } else {
                    format!("{}({}, {})", sig, ccode, arg_codes.join(", "))
                };
                (call, ret, fresh)
            }
            Some(CallTarget::ToCStr) => {
                let (code, _, _) = self.gen_expr(callee_as_recv(callee), g);
                (format!("((void*)({})->bytes)", code), Ty::Ptr, false)
            }
            Some(CallTarget::ToStr) => {
                let (code, t, _) = self.gen_expr(callee_as_recv(callee), g);
                let c = match t {
                    Ty::Int => format!("fang_str_from_i64({})", code),
                    Ty::Float => format!("fang_str_from_f64({})", code),
                    Ty::Bool => format!("fang_str_from_bool({})", code),
                    _ => code,
                };
                (c, Ty::Str, true)
            }
            None => ("NULL".into(), Ty::Err, false),
        }
    }

    fn gen_method(
        &mut self,
        recv: &Expr,
        name: &str,
        args: &[Expr],
        call_expr: &Expr,
        g: &Gen,
    ) -> (String, Ty, bool) {
        // str module functions resolved through the normal fn machinery
        let target = self
            .a
            .call
            .get(&(self.module.clone(), call_expr.span.start))
            .cloned();
        // Qualified module call: math.sqrt(x) parses as a method; the
        // receiver is the module name and is not passed.
        if let Some(CallTarget::Fn(mangled)) = &target {
            let fi = g.a.fns[mangled].clone();
            let mut arg_codes = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                let pt = fi.params.get(i).map(|p| p.ty.clone()).unwrap_or(Ty::Err);
                let (code, _, fr) = self.gen_expr(arg, g);
                if g.is_rc(&pt) && !fr {
                    self.w(&format!("fang_incref((void*)({}));", code));
                }
                arg_codes.push(code);
            }
            let c_name = if fi.is_extern { fi.c_name.clone() } else { mangled.clone() };
            let ret_ty = self
                .a
                .ty
                .get(&(self.module.clone(), call_expr.span.start))
                .cloned()
                .unwrap_or(Ty::Err);
            let fresh = g.is_rc(&ret_ty);
            return (
                format!("{}({})", c_name, arg_codes.join(", ")),
                ret_ty,
                fresh,
            );
        }
        if let Some(CallTarget::FnSelf(mangled)) = target {
            let fi = g.a.fns[&mangled].clone();
            let (recv_code, _, fresh) = self.gen_expr(recv, g);
            if !fresh {
                let pt = fi.params.first().map(|p| p.ty.clone()).unwrap_or(Ty::Err);
                if g.is_rc(&pt) {
                    self.w(&format!("fang_incref((void*)({}));", recv_code));
                }
            }
            let mut arg_codes = vec![recv_code];
            for (i, arg) in args.iter().enumerate() {
                let pt = fi.params.get(i + 1).map(|p| p.ty.clone()).unwrap_or(Ty::Err);
                let (code, _, fr) = self.gen_expr(arg, g);
                if g.is_rc(&pt) && !fr {
                    self.w(&format!("fang_incref((void*)({}));", code));
                }
                arg_codes.push(code);
            }
            let ret_ty = self
                .a
                .ty
                .get(&(self.module.clone(), call_expr.span.start))
                .cloned()
                .unwrap_or(Ty::Err);
            let fresh = g.is_rc(&ret_ty);
            return (
                format!("{}({})", mangled, arg_codes.join(", ")),
                ret_ty,
                fresh,
            );
        }
        // builtin methods route through gen_call with the receiver as callee
        let _ = name;
        self.gen_call(&make_recv_expr(recv), args, call_expr, g)
    }
}

/// For method sugar builtins, the parser kept the receiver inside the Method
/// node; gen_call receives it wrapped as a pseudo callee.
fn callee_as_recv(callee: &Expr) -> &Expr {
    callee
}

fn make_recv_expr(recv: &Expr) -> Expr {
    recv.clone()
}

/// Print-debug kind codes for the runtime renderer.
fn debug_kind(elem: &Ty) -> i32 {
    match elem {
        Ty::Str => 0,
        Ty::Int => 1,
        Ty::Float => 2,
        Ty::Bool => 4,
        _ => 3,
    }
}

fn emit_body_with_trailing(fg: &mut crate::codegen::FnGen, body: &[Stmt], ret: &Ty, g: &crate::codegen::Gen) {
    // Comments attach to the previous block; the trailing expression is
    // the last non-comment statement.
    let last_idx = body
        .iter()
        .rposition(|st| !matches!(st.kind, StmtKind::Comment(_)));
    let last_idx = match last_idx {
        Some(i) => i,
        None => {
            fg.stmts(body, g);
            return;
        }
    };
    let last = &body[last_idx];
    let prefix = &body[..last_idx];
    let implicit_ret = !matches!(ret, Ty::Void | Ty::Err)
        && matches!(&last.kind, StmtKind::Expr(_));
    if !implicit_ret {
        fg.stmts(body, g);
        return;
    }
    // One scope for the whole body, exactly like stmts(): the trailing
    // expression must still see the locals the prefix statements declared.
    fg.scopes.push(std::collections::HashMap::new());
    for st in prefix {
        fg.stmt(st, g);
    }
    if let StmtKind::Expr(e) = &last.kind {
        fg.line_of(last);
        let (code, _t, fresh) = fg.gen_expr(e, g);
        if g.is_rc(ret) && !fresh {
            fg.w(&format!("fang_incref((void*)({}));", code));
        }
        if let Ty::Struct(m) = ret {
            fg.w(&format!("{} __r = {};", g.ctype(ret), code));
            for si in g.struct_field_incref("__r", m) {
                fg.w(&si);
            }
            fg.w("__ret = __r; goto __exit;");
        } else {
            fg.w(&format!("__ret = {}; goto __exit;", code));
        }
        fg.flush_temps(g);
    }
    fg.scopes.pop();
}

fn default_value(t: &Ty) -> String {
    match t {
        Ty::Int | Ty::Float | Ty::Bool => "0".into(),
        _ => "NULL".into(),
    }
}

fn promote(lc: &str, lt: &Ty, promoted: &Ty, rc: &str, rt: &Ty) -> (String, String) {
    if *promoted == Ty::Float {
        let l = if *lt == Ty::Int {
            format!("((double){})", lc)
        } else {
            lc.to_string()
        };
        let r = if *rt == Ty::Int {
            format!("((double){})", rc)
        } else {
            rc.to_string()
        };
        (l, r)
    } else {
        (lc.to_string(), rc.to_string())
    }
}

fn struct_order(a: &Analysis) -> Vec<String> {
    // Dependency order: a struct listed after the structs it contains.
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    for si in a.structs.values() {
        let d: Vec<String> = si
            .fields
            .iter()
            .filter_map(|(_, t)| match t {
                Ty::Struct(m) => Some(m.clone()),
                _ => None,
            })
            .collect();
        deps.insert(si.mangled.clone(), d);
    }
    let mut order = Vec::new();
    let mut done = HashSet::new();
    let all: Vec<String> = a.structs.keys().cloned().collect();
    for s in &all {
        visit_struct(s, &deps, &mut done, &mut order);
    }
    order
}

fn visit_struct(s: &str, deps: &HashMap<String, Vec<String>>, done: &mut HashSet<String>, order: &mut Vec<String>) {
    if done.contains(s) {
        return;
    }
    done.insert(s.to_string());
    if let Some(ds) = deps.get(s) {
        for d in ds.clone() {
            visit_struct(&d, deps, done, order);
        }
    }
    order.push(s.to_string());
}

fn collect_strs(m: &Module, pool: &mut HashMap<String, String>, order: &mut Vec<String>) {
    let mut add = |s: &str, pool: &mut HashMap<String, String>, order: &mut Vec<String>| {
        if !s.is_empty() && !pool.contains_key(s) {
            order.push(s.to_string());
        }
        let _ = pool;
    };
    let _ = &mut add;
    for item in &m.items {
        match item {
            Item::Const { value, .. } => collect_expr_strs(value, order),
            Item::Fn(f) => {
                for p in &f.params {
                    if let Some(d) = &p.default {
                        collect_expr_strs(d, order);
                    }
                }
                for st in &f.body {
                    collect_stmt_strs(st, order);
                }
            }
            _ => {}
        }
    }
    let _ = pool;
}

fn collect_stmt_strs(st: &Stmt, order: &mut Vec<String>) {
    match &st.kind {
        StmtKind::Let { value, .. } => collect_expr_strs(value, order),
        StmtKind::Assign { target, value } => {
            collect_lvalue_strs(target, order);
            collect_expr_strs(value, order);
        }
        StmtKind::TypedAssign { value, .. } => collect_expr_strs(value, order),
        StmtKind::AssignMulti { value, .. } => collect_expr_strs(value, order),
        StmtKind::Return(Some(e)) => collect_expr_strs(e, order),
        StmtKind::Raise(e) => collect_expr_strs(e, order),
        StmtKind::Expr(e) => collect_expr_strs(e, order),
        StmtKind::If { cond, then, elifs, els } => {
            collect_expr_strs(cond, order);
            for s in then {
                collect_stmt_strs(s, order);
            }
            for (c, b) in elifs {
                collect_expr_strs(c, order);
                for s in b {
                    collect_stmt_strs(s, order);
                }
            }
            if let Some(b) = els {
                for s in b {
                    collect_stmt_strs(s, order);
                }
            }
        }
        StmtKind::While { cond, body } => {
            collect_expr_strs(cond, order);
            for s in body {
                collect_stmt_strs(s, order);
            }
        }
        StmtKind::For { body, .. } => {
            for s in body {
                collect_stmt_strs(s, order);
            }
        }
        StmtKind::Try { body, handler, .. } => {
            for s in body {
                collect_stmt_strs(s, order);
            }
            for s in handler {
                collect_stmt_strs(s, order);
            }
        }
        _ => {}
    }
}

fn collect_lvalue_strs(lv: &LValue, order: &mut Vec<String>) {
    match lv {
        LValue::Ident(_) => {}
        LValue::Index(_, idx) => collect_expr_strs(idx, order),
        LValue::Field(_, _) => {}
    }
}

fn collect_expr_strs(e: &Expr, order: &mut Vec<String>) {
    match &e.kind {
        ExprKind::Str(s) => {
            if !order.contains(s) {
                order.push(s.clone());
            }
        }
        ExprKind::FStr(parts) => {
            for p in parts {
                if let FStrPartAst::Lit(l) = p {
                    if !order.contains(l) {
                        order.push(l.clone());
                    }
                }
                if let FStrPartAst::E(sub) = p {
                    collect_expr_strs(sub, order);
                }
            }
        }
        ExprKind::Unary(_, i) => collect_expr_strs(i, order),
        ExprKind::Binary(_, l, r) => {
            collect_expr_strs(l, order);
            collect_expr_strs(r, order);
        }
        ExprKind::And(l, r) | ExprKind::Or(l, r) => {
            collect_expr_strs(l, order);
            collect_expr_strs(r, order);
        }
        ExprKind::Not(i) => collect_expr_strs(i, order),
        ExprKind::Compare(_, l, r) => {
            collect_expr_strs(l, order);
            collect_expr_strs(r, order);
        }
        ExprKind::Call(callee, args) => {
            collect_expr_strs(callee, order);
            for a in args {
                collect_expr_strs(a, order);
            }
        }
        ExprKind::Method(recv, _, args) => {
            collect_expr_strs(recv, order);
            for a in args {
                collect_expr_strs(a, order);
            }
        }
        ExprKind::Field(r, _) => collect_expr_strs(r, order),
        ExprKind::Index(r, idx) => {
            collect_expr_strs(r, order);
            collect_expr_strs(idx, order);
        }
        ExprKind::Slice(r, s, en) => {
            collect_expr_strs(r, order);
            if let Some(x) = s {
                collect_expr_strs(x, order);
            }
            if let Some(x) = en {
                collect_expr_strs(x, order);
            }
        }
        ExprKind::ListLit(items) => {
            for i in items {
                collect_expr_strs(i, order);
            }
        }
        ExprKind::DictLit(pairs) => {
            for (k, v) in pairs {
                collect_expr_strs(k, order);
                collect_expr_strs(v, order);
            }
        }
        ExprKind::TupleLit(a, b) => {
            collect_expr_strs(a, order);
            collect_expr_strs(b, order);
        }
        ExprKind::Cast(_, i) => collect_expr_strs(i, order),
        ExprKind::FnExpr(f) => {
            for st in &f.body {
                collect_stmt_strs(st, order);
            }
        }
        ExprKind::Comprehension { item, binders, iter, cond } => {
            collect_expr_strs(item, order);
            let _ = binders;
            match &**iter {
                Iter::Expr(ie) => collect_expr_strs(ie, order),
                Iter::Range { start, end, .. } => {
                    collect_expr_strs(start, order);
                    collect_expr_strs(end, order);
                }
            }
            if let Some(c) = cond {
                collect_expr_strs(c, order);
            }
        }
        _ => {}
    }
}

fn c_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 32 => out.push_str(&format!("\\{:03o}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

impl<'a> Gen<'a> {
    /// Stamp (once per signature) a trampoline that calls through a
    /// closure's function pointer with the static signature.
    fn stamp_trampoline(&self, key: &str, params: &[Ty], ret: &Ty) -> String {
        let sig_name = format!("fang_call_{}", key);
        if self.stamped_sigs.borrow().contains(&sig_name) {
            return sig_name;
        }
        self.stamped_sigs.borrow_mut().insert(sig_name.clone());
        let mut c_params = vec!["fang_closure* c".to_string()];
        for (i, p) in params.iter().enumerate() {
            c_params.push(format!("{} p{}", self.ctype(p), i));
        }
        let arglist = (0..params.len())
            .map(|i| format!("p{}", i))
            .collect::<Vec<_>>()
            .join(", ");
        let ret_c = self.ctype_ret(ret);
        let fn_ty = format!(
            "{} (*)(void*{}{})",
            ret_c,
            if params.is_empty() { "" } else { ", " },
            params.iter().map(|p| self.ctype(p)).collect::<Vec<_>>().join(", ")
        );
        if matches!(ret, Ty::Void | Ty::Err) {
            writeln!(
                self.extra_decls.borrow_mut(),
                "static void {name}({params}) {{\n    (({fn_ty})c->fnptr)(c->env{comma}{args});\n}}",
                name = sig_name,
                params = c_params.join(", "),
                fn_ty = fn_ty,
                comma = if params.is_empty() { "" } else { ", " },
                args = arglist,
            )
            .unwrap();
        } else {
            writeln!(
                self.extra_decls.borrow_mut(),
                "static {ret} {name}({params}) {{\n    return (({fn_ty})c->fnptr)(c->env{comma}{args});\n}}",
                ret = ret_c,
                name = sig_name,
                params = c_params.join(", "),
                fn_ty = fn_ty,
                comma = if params.is_empty() { "" } else { ", " },
                args = arglist,
            )
            .unwrap();
        }
        sig_name
    }

    fn pool_str(&self, s: &str) -> String {
        if let Some(name) = self.pool.get(s) {
            return name.clone();
        }
        // pool filled during collect; look up by index
        let idx = self.pool_order.iter().position(|x| x == s).unwrap_or(usize::MAX);
        format!("L{}", idx)
    }
}
