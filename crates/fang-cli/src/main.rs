//! The `fang` binary: run, build, repl, fmt, test, add.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use fang_core::{parse_source, sema, Analysis, GenOpts, Module, SourceUnit};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const USAGE: &str = "\
fang - looks like Python, runs like C

USAGE:
    fang <COMMAND> [ARGS]

COMMANDS:
    run <file.fang> [--release] [--keep-c] [--freestanding] [--python]
                          compile and execute a Fang program
    build <file.fang> [--release] [-o name] [--freestanding] [--target TRIPLE]
                          produce a standalone native binary
    repl                  interactive session (replays the session each input)
    learn                 interactive course, starts you from zero
    new <name>            create a starter project
    cheat                 the whole language on one screen
    doc <topic>           instant help for any word
    fmt <files...>        format source files in the canonical style
    test [dir]            discover and run *_test.fang files
    add <repo>            install a package from a git repository
    version               print the version
    help                  print this help

New to programming? Start with:  fang learn

The toolchain needs cc (gcc or clang) on PATH.

EXIT CODES:
    0    success
    1    compile error or CLI misuse
    2    test failure
    101  runtime panic in a Fang program
";

/// The C runtime, embedded so the fang binary is self-contained.
const RT_H: &str = include_str!("../../../runtime/fang_rt.h");
const RT_C: &str = include_str!("../../../runtime/fang_rt.c");
const RT_PY_H: &str = include_str!("../../../runtime/fang_py.h");
const RT_PY_C: &str = include_str!("../../../runtime/fang_py.c");

fn output_of(prog: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(prog)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run {} ({}): Python embedding needs the python3 dev package", prog, e))?;
    if !out.status.success() {
        return Err(format!("{} failed", prog));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Standard library modules, embedded at build time.
static STD_MODULES: &[(&str, &str)] = &[
    ("io", include_str!("../../../std/io.fang")),
    ("str", include_str!("../../../std/str.fang")),
    ("math", include_str!("../../../std/math.fang")),
    ("os", include_str!("../../../std/os.fang")),
    ("fs", include_str!("../../../std/fs.fang")),
    ("time", include_str!("../../../std/time.fang")),
    ("json", include_str!("../../../std/json.fang")),
    ("net", include_str!("../../../std/net.fang")),
    ("py", include_str!("../../../std/py.fang")),
    ("server", include_str!("../../../std/server.fang")),
];

use std::collections::HashMap;

mod cheat;
mod learn;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("help") {
        "version" | "--version" | "-V" => {
            println!("fang {}", VERSION);
            ExitCode::SUCCESS
        }
        "help" | "--help" | "-h" => {
            print!("{}", USAGE);
            ExitCode::SUCCESS
        }
        "run" => cmd_run(&args[1..]),
        "build" => cmd_build(&args[1..]),
        "fmt" => cmd_fmt(&args[1..]),
        "repl" => cmd_repl(),
        "test" => cmd_test(&args[1..]),
        "add" => cmd_add(&args[1..]),
        "learn" => learn::cmd_learn(&args[1..]),
        "new" => learn::cmd_new(&args[1..]),
        "cheat" => cheat::cmd_cheat(),
        "doc" => cheat::cmd_doc(&args[1..]),
        cmd => {
            eprintln!("fang: unknown or unfinished command '{}'", cmd);
            eprint!("{}", USAGE);
            ExitCode::from(1)
        }
    }
}

struct BuildFlags {
    release: bool,
    keep_c: bool,
    freestanding: bool,
    python: bool,
    target: Option<String>,
    output: Option<String>,
}

fn parse_flags(args: &[String]) -> Result<(String, BuildFlags), String> {
    let mut file: Option<String> = None;
    let mut flags = BuildFlags {
        release: false,
        keep_c: false,
        freestanding: false,
        python: false,
        target: None,
        output: None,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--release" => flags.release = true,
            "--keep-c" => flags.keep_c = true,
            "--freestanding" => flags.freestanding = true,
            "--python" => flags.python = true,
            "--target" => {
                i += 1;
                flags.target = Some(
                    args.get(i)
                        .ok_or("--target needs a triple like x86_64-elf")?
                        .clone(),
                );
            }
            "-o" => {
                i += 1;
                flags.output = Some(args.get(i).ok_or("-o needs a name")?.clone());
            }
            other => {
                if other.starts_with('-') {
                    return Err(format!("unknown flag '{}'", other));
                }
                if file.is_some() {
                    return Err(format!("unexpected argument '{}'", other));
                }
                file = Some(other.to_string());
            }
        }
        i += 1;
    }
    let file = file.ok_or("pass a .fang file")?;
    if !file.ends_with(".fang") {
        return Err(format!("'{}' is not a .fang file", file));
    }
    Ok((file, flags))
}

/// Std units parsed once per program build. The Python bridge module
/// is excluded: it carries externs that need libpython, so it loads
/// only when explicitly imported.
fn embedded_std_units() -> Vec<SourceUnit> {
    STD_MODULES
        .iter()
        .filter(|(name, _)| *name != "py")
        .filter_map(|(name, src)| {
            parse_source(name, src)
                .ok()
                .map(|m| SourceUnit {
                    name: name.to_string(),
                    path: format!("<std>/{}.fang", name),
                    module: m,
                })
        })
        .collect()
}

/// Load a program: the entry file plus transitive imports (stdlib is
/// embedded; local files and packages resolve from disk).
fn load_program(entry: &str) -> Result<Vec<SourceUnit>, String> {
    let entry_path = PathBuf::from(entry);
    let name = entry_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("bad entry file name")?
        .to_string();
    let src = std::fs::read_to_string(&entry_path)
        .map_err(|e| format!("cannot read {}: {}", entry, e))?;
    if STD_MODULES.iter().any(|(n, _)| *n == name) {
        return Err(format!(
            "the entry file '{}' collides with the std module '{}'; rename the file",
            entry, name
        ));
    }
    let module = parse_source(&name, &src)?;
    // Std modules are always present: method sugar (s.upper(), xs.push)
    // resolves through them, while qualified use (math.sqrt) still needs
    // its import statement.
    let mut units = embedded_std_units();
    let mut seen: Vec<String> = STD_MODULES
        .iter()
        .filter(|(n, _)| *n != "py")
        .map(|(n, _)| n.to_string())
        .collect();
    load_imports(module, name, entry_path.to_string_lossy().to_string(), &mut units, &mut seen, &entry_path)?;
    Ok(units)
}

fn load_imports(
    module: Module,
    name: String,
    path: String,
    units: &mut Vec<SourceUnit>,
    seen: &mut Vec<String>,
    base_dir: &Path,
) -> Result<(), String> {
    if seen.contains(&name) {
        return Err(format!("cyclic import of module '{}'", name));
    }
    seen.push(name.clone());
    for item in &module.items {
        if let fang_core::ast::Item::Import { module: imp, path: ipath, .. } = item {
            if seen.iter().any(|s| s == imp) {
                continue; // already loaded
            }
            // 1. embedded std module
            if let Some((_, src)) = STD_MODULES.iter().find(|(n, _)| n == imp) {
                let m = parse_source(imp, src)?;
                load_imports(m, imp.clone(), format!("<std>/{}.fang", imp), units, seen, base_dir)?;
                continue;
            }
            // 2. explicit relative path
            if let Some(p) = ipath {
                let resolved = base_dir.parent().unwrap_or(Path::new(".")).join(p.trim_start_matches("./"));
                let src = std::fs::read_to_string(&resolved)
                    .map_err(|e| format!("cannot read import '{}': {}", p, e))?;
                let m = parse_source(imp, &src)?;
                load_imports(m, imp.clone(), resolved.to_string_lossy().to_string(), units, seen, base_dir)?;
                continue;
            }
            // 3. sibling file <name>.fang
            let sibling = base_dir.parent().unwrap_or(Path::new(".")).join(format!("{}.fang", imp));
            // 4. package
            let pkg = base_dir
                .parent()
                .unwrap_or(Path::new("."))
                .join("fang_packages")
                .join(imp)
                .join(format!("{}.fang", imp));
            let resolved = if sibling.exists() {
                sibling
            } else if pkg.exists() {
                pkg
            } else {
                return Err(format!(
                    "unknown module '{}'; not a std module, a local {}.fang, or a package",
                    imp, imp
                ));
            };
            let src = std::fs::read_to_string(&resolved)
                .map_err(|e| format!("cannot read import '{}': {}", imp, e))?;
            let m = parse_source(imp, &src)?;
            load_imports(
                m,
                imp.clone(),
                resolved.to_string_lossy().to_string(),
                units,
                seen,
                base_dir,
            )?;
        }
    }
    units.push(SourceUnit {
        name,
        path,
        module,
    });
    Ok(())
}

/// Source text per module name, for caret diagnostics.
fn source_map(units: &[SourceUnit]) -> HashMap<String, String> {
    units
        .iter()
        .map(|u| (u.name.clone(), std::fs::read_to_string(&u.path).unwrap_or_default()))
        .collect()
}

/// Render an error with the source line and a caret.
fn render_error(e: &sema::SemaError, sources: &HashMap<String, String>, paths: &HashMap<String, String>) {
    let path = paths.get(&e.module).cloned().unwrap_or_else(|| e.module.clone());
    let line_src = sources
        .get(&e.module)
        .and_then(|s| s.lines().nth((e.line.max(1) - 1) as usize))
        .unwrap_or("");
    eprintln!("{}:{}:{}: error: {}", path, e.line, e.col, e.msg);
    let ln = e.line.max(1).to_string();
    let pad = " ".repeat(ln.len());
    let shown = line_src.trim_start_matches(' ');
    eprintln!("{} | {}", ln, shown);
    let caret_col = if e.col > 0 { e.col as usize - 1 } else { 0 };
    let trimmed = line_src.len() - shown.len();
    let caret_at = caret_col.saturating_sub(trimmed);
    eprintln!("{} | {}^", pad, " ".repeat(caret_at));
}

fn analyze_program(units: &[SourceUnit], entry: &str) -> Result<Analysis, String> {
    let sources: Vec<(String, Module)> = units
        .iter()
        .map(|u| (u.name.clone(), u.module.clone()))
        .collect();
    let a = sema::analyze(sources, entry);
    if a.errors.is_empty() {
        return Ok(a);
    }
    let srcs = source_map(units);
    let paths: HashMap<String, String> = units
        .iter()
        .map(|u| (u.name.clone(), u.path.clone()))
        .collect();
    for e in a.errors.iter().take(10) {
        render_error(e, &srcs, &paths);
    }
    Err("compile error".to_string())
}

struct Compiled {
    binary: PathBuf,
}

fn compile_program(
    units: &[SourceUnit],
    a: &Analysis,
    entry: &str,
    flags: &BuildFlags,
    tag: &str,
) -> Result<Compiled, String> {
    let opts = GenOpts {
        freestanding: flags.freestanding,
    };
    let mut c_src = fang_core::codegen::generate(units, a, entry, &opts)?;
    if flags.python {
        c_src = c_src.replace(
            "int main(int argc, char** argv) {",
            "#include <fang_py.h>\nint main(int argc, char** argv) {\n    fang_py_init();",
        );
    }

    let build_dir = std::env::temp_dir().join(format!("fang-build-{}", tag));
    std::fs::create_dir_all(&build_dir).map_err(|e| format!("cannot create build dir: {}", e))?;
    let c_path = build_dir.join("program.c");
    std::fs::write(&c_path, &c_src).map_err(|e| format!("cannot write C: {}", e))?;
    std::fs::write(build_dir.join("fang_rt.h"), RT_H).map_err(|e| e.to_string())?;
    std::fs::write(build_dir.join("fang_rt.c"), RT_C).map_err(|e| e.to_string())?;
    let py_cflags = if flags.python {
        output_of("python3-config", &["--embed", "--cflags"])?
    } else {
        String::new()
    };
    if flags.python {
        std::fs::write(build_dir.join("fang_py.h"), RT_PY_H).map_err(|e| e.to_string())?;
        std::fs::write(build_dir.join("fang_py.c"), RT_PY_C).map_err(|e| e.to_string())?;
    }

    if flags.keep_c {
        if let Some(dir) = Path::new(&units.last().unwrap().path).parent() {
            let _ = std::fs::write(dir.join("program.fang.c"), &c_src);
        }
    }

    let out_name = flags.output.clone().unwrap_or_else(|| {
        let stem = Path::new(&units.last().unwrap().path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("program")
            .to_string();
        if flags.freestanding {
            format!("{}.elf", stem)
        } else {
            format!("{}.bin", stem)
        }
    });
    let binary = build_dir.join(&out_name);

    let opt = if flags.release { "-O3" } else { "-O0" };
    let mut cmd = Command::new("cc");
    cmd.arg(opt)
        .arg("-std=c11")
        .arg("-I")
        .arg(&build_dir)
        .arg(&c_path);
    if flags.release {
        cmd.arg("-march=native");
    }
    if !flags.freestanding {
        cmd.arg(build_dir.join("fang_rt.c"));
    }
    if flags.python {
        for flag in py_cflags.split_whitespace() {
            cmd.arg(flag);
        }
        cmd.arg(build_dir.join("fang_py.c"));
    }
    if let Some(triple) = &flags.target {
        cmd.arg(format!("--target={}", triple));
    }
    if flags.freestanding {
        cmd.arg("-nostdlib").arg("-ffreestanding").arg("-fno-builtin");
    } else {
        cmd.arg("-lm");
    }
    cmd.arg("-o").arg(&binary);
    let py_ldflags = if flags.python {
        output_of("python3-config", &["--embed", "--ldflags"])?
    } else {
        String::new()
    };
    if flags.python {
        for flag in py_ldflags.split_whitespace() {
            cmd.arg(flag);
        }
    }
    let status = cmd
        .status()
        .map_err(|e| format!("cannot run cc: {}. Is a C compiler installed?", e))?;
    if !status.success() {
        return Err("C compilation failed".to_string());
    }
    Ok(Compiled { binary })
}

fn cmd_run(args: &[String]) -> ExitCode {
    let (file, flags) = match parse_flags(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("fang run: {}", e);
            return ExitCode::from(1);
        }
    };
    match run_program(&file, &flags, "run") {
        Ok(code) => code,
        Err(e) => {
            eprintln!("fang: {}", e);
            ExitCode::from(1)
        }
    }
}

fn cmd_build(args: &[String]) -> ExitCode {
    let (file, flags) = match parse_flags(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("fang build: {}", e);
            return ExitCode::from(1);
        }
    };
    match build_program(&file, &flags) {
        Ok(path) => {
            println!("built: {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("fang: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run_program(file: &str, flags: &BuildFlags, tag: &str) -> Result<ExitCode, String> {
    let units = load_program(file)?;
    let entry = units.last().unwrap().name.clone();
    let a = analyze_program(&units, &entry)?;
    let compiled = compile_program(&units, &a, &entry, flags, tag)?;
    let status = Command::new(&compiled.binary)
        .status()
        .map_err(|e| format!("cannot run the compiled program: {}", e))?;
    let code = status.code().unwrap_or(1);
    Ok(ExitCode::from(code.clamp(0, 255) as u8))
}

fn build_program(file: &str, flags: &BuildFlags) -> Result<PathBuf, String> {
    let units = load_program(file)?;
    let entry = units.last().unwrap().name.clone();
    let a = analyze_program(&units, &entry)?;
    let tag = format!("build-{}", std::process::id());
    let compiled = compile_program(&units, &a, &entry, flags, &tag)?;
    // Absolute -o paths are honored as-is; anything else lands next to
    // the source with a .bin suffix.
    let out = match &flags.output {
        Some(o) if Path::new(o).is_absolute() => PathBuf::from(o),
        Some(o) => Path::new(file).with_file_name(o),
        None => Path::new(file).with_extension("bin"),
    };
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if out != compiled.binary {
        std::fs::copy(&compiled.binary, &out).map_err(|e| format!("cannot write output: {}", e))?;
    }
    Ok(out)
}

fn cmd_fmt(files: &[String]) -> ExitCode {
    if files.is_empty() {
        eprintln!("fang fmt: pass at least one .fang file");
        return ExitCode::from(1);
    }
    let mut failed = false;
    for path in files {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("fang fmt: cannot read {}: {}", path, e);
                failed = true;
                continue;
            }
        };
        match parse_source("fmt", &src) {
            Ok(m) => {
                let out = fang_core::format_module(&m);
                if out != src {
                    if let Err(e) = std::fs::write(path, &out) {
                        eprintln!("fang fmt: cannot write {}: {}", path, e);
                        failed = true;
                        continue;
                    }
                    println!("formatted: {}", path);
                } else {
                    println!("unchanged: {}", path);
                }
            }
            Err(e) => {
                eprintln!("fang fmt: {}: {}", path, e);
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}


// ---------------------------------------------------------------------
// REPL
// ---------------------------------------------------------------------

/// Session state: top-level definitions plus interactive statements that
/// live inside a regenerated fn main().
struct ReplSession {
    defs: String,
    stmts: String,
}

impl ReplSession {
    fn program(&self) -> String {
        let mut out = self.defs.clone();
        if !self.defs.is_empty() {
            out.push('\n');
        }
        out.push_str("fn main():\n");
        if self.stmts.is_empty() {
            out.push_str("  pass\n");
        } else {
            out.push_str(&self.stmts);
        }
        out
    }
}

/// Handle a single REPL input line (also used for the line that closes a
/// definition block).
fn process_repl_line(
    rl: &mut rustyline::DefaultEditor,
    session: &mut ReplSession,
    line: &str,
    prev_output: &mut Vec<String>,
    dir: &Path,
) {
    let _ = rl;
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    if line.starts_with("let ") || line.contains(" = ") {
        session.stmts.push_str("  ");
        session.stmts.push_str(line);
        session.stmts.push('\n');
    } else {
        session.stmts.push_str(&format!("  print({})\n", line));
        session.stmts.push_str("  print()\n");
    }
    let program = session.program();
    match try_run_repl(&program, dir) {
        Ok(out) => {
            let lines: Vec<String> = out.lines().map(String::from).collect();
            for l in lines.iter().skip(prev_output.len()) {
                println!("{}", l);
            }
            *prev_output = lines;
        }
        Err(e) => {
            eprintln!("{}", e);
            let back = if line.starts_with("let ") || line.contains(" = ") {
                1
            } else {
                2
            };
            session.stmts = drop_last_lines(&session.stmts, back);
        }
    }
}

fn cmd_repl() -> ExitCode {
    println!("fang {} repl", VERSION);
    println!("definitions and statements accumulate; each input recompiles and");
    println!("reruns the whole session, so new output appears below the replay.");
    println!("exit with ctrl-d");
    let mut rl = rustyline::DefaultEditor::new().expect("terminal");
    let dir = std::env::temp_dir().join(format!("fang-repl-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let mut session = ReplSession {
        defs: String::new(),
        stmts: String::new(),
    };
    let mut prev_output: Vec<String> = Vec::new();
    while let Ok(raw) = rl.readline(">>> ") {
        let line = raw;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        rl.add_history_entry(line).ok();
        if line == "exit" || line == "quit" {
            break;
        }
        let is_def = line.starts_with("fn ")
            || line.starts_with("struct ")
            || line.starts_with("import ")
            || line.starts_with("extern ")
            || line.starts_with("# ");
        if is_def {
            session.defs.push_str(line);
            session.defs.push('\n');
            // A definition header opens a block: read indented lines until
            // a line starts at column zero.
            if line.ends_with(':') && !line.starts_with("import ") {
                'def_block: {
                    while let Ok(cont) = rl.readline("... ") {
                        let cont = cont.trim_end();
                        if cont.trim().is_empty() {
                            session.defs.push('\n');
                            continue;
                        }
                        if !cont.starts_with(' ') && !cont.starts_with('\t') {
                            // End of block; process this line as new input.
                            rl.add_history_entry(cont).ok();
                            process_repl_line(&mut rl, &mut session, cont, &mut prev_output, &dir);
                            break 'def_block;
                        }
                        session.defs.push_str(cont);
                        session.defs.push('\n');
                    }
                }
            }
        } else if line.starts_with("let ") || line.contains(" = ") {
            session.stmts.push_str("  ");
            session.stmts.push_str(line);
            session.stmts.push('\n');
        } else {
            // Bare expression: echo its value.
            session.stmts.push_str(&format!("  print({})\n", line));
            session.stmts.push_str("  print()\n");
        }
        let program = session.program();
        match try_run_repl(&program, &dir) {
            Ok(out) => {
                let lines: Vec<String> = out.lines().map(String::from).collect();
                for l in lines.iter().skip(prev_output.len()) {
                    println!("{}", l);
                }
                prev_output = lines;
            }
            Err(e) => {
                eprintln!("{}", e);
                // Roll the failed input back so the session stays usable.
                if is_def {
                    session.defs = drop_last_lines(&session.defs, 1);
                } else {
                    let back = if line.starts_with("let ") || line.contains(" = ") {
                        1
                    } else {
                        2
                    };
                    session.stmts = drop_last_lines(&session.stmts, back);
                }
            }
        }
    }
    println!("bye");
    ExitCode::SUCCESS
}

fn drop_last_lines(s: &str, n: usize) -> String {
    let mut lines: Vec<&str> = s.lines().collect();
    let keep = lines.len().saturating_sub(n);
    lines.truncate(keep);
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn try_run_repl(program: &str, dir: &Path) -> Result<String, String> {
    let session_path = dir.join("repl.fang");
    std::fs::write(&session_path, program).map_err(|e| e.to_string())?;
    let m = parse_source("repl", program).map_err(|e| format!("parse: {}", e))?;
    let mut units = embedded_std_units();
    units.push(SourceUnit {
        name: "repl".to_string(),
        path: session_path.to_string_lossy().to_string(),
        module: m,
    });
    let a = analyze_program(&units, "repl")?;
    let flags = BuildFlags {
        release: false,
        keep_c: false,
        freestanding: false,
        python: false,
        target: None,
        output: Some("repl.bin".to_string()),
    };
    let tag = format!("repl-{}", std::process::id());
    let compiled = compile_program(&units, &a, "repl", &flags, &tag)?;
    let out = Command::new(&compiled.binary)
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("runtime error (exit {})", out.status.code().unwrap_or(-1))
        } else {
            err
        });
    }
    Ok(text)
}

// ---------------------------------------------------------------------
// test runner
// ---------------------------------------------------------------------

fn cmd_test(args: &[String]) -> ExitCode {
    let dir = args.first().cloned().unwrap_or_else(|| ".".to_string());
    let mut tests = Vec::new();
    collect_tests(std::path::Path::new(&dir), &mut tests);
    if tests.is_empty() {
        eprintln!("fang test: no *_test.fang files found under {}", dir);
        return ExitCode::from(1);
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("fang"));
    let fang = exe
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("target/debug/fang"))
        .filter(|p| p.exists())
        .unwrap_or(exe);
    let mut failed = 0;
    for t in &tests {
        let out = Command::new(&fang).arg("run").arg(t).output().expect("run fang");
        if out.status.success() {
            println!("PASS  {}", t.display());
        } else {
            failed += 1;
            eprintln!("FAIL  {}", t.display());
            let err = String::from_utf8_lossy(&out.stderr);
            for line in err.lines().take(6) {
                eprintln!("      {}", line);
            }
        }
    }
    println!("{} test(s), {} failed", tests.len(), failed);
    if failed > 0 {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn collect_tests(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "fang_packages" && !name.starts_with('.') {
                    collect_tests(&p, out);
                }
            } else if p.extension().is_some_and(|x| x == "fang")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with("_test.fang"))
            {
                out.push(p);
            }
        }
    }
}

// ---------------------------------------------------------------------
// package manager
// ---------------------------------------------------------------------

fn cmd_add(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("fang add: usage: fang add <git-url | user/repo>");
        return ExitCode::from(1);
    }
    let repo = &args[0];
    let local = repo.starts_with('/') || repo.starts_with("./") || repo.starts_with("../");
    let url = if repo.contains("://") || repo.ends_with(".git") || local {
        repo.clone()
    } else {
        format!("https://github.com/{}.git", repo)
    };
    let name = url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or("package")
        .to_string();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        eprintln!("fang add: cannot derive a package name from '{}'", repo);
        return ExitCode::from(1);
    }
    let pkg_dir = std::path::Path::new("fang_packages").join(&name);
    if pkg_dir.exists() {
        eprintln!("fang add: package '{}' is already installed", name);
        return ExitCode::from(1);
    }
    println!("fetching {} ...", url);
    let status = Command::new("git")
        .args(["clone", "--depth", "1", &url, pkg_dir.to_str().unwrap()])
        .status();
    let status = match status {
        Ok(s) => s,
        Err(_) => {
            eprintln!("fang add: git is required to install packages");
            return ExitCode::from(1);
        }
    };
    if !status.success() {
        let _ = std::fs::remove_dir_all(&pkg_dir);
        eprintln!("fang add: clone failed");
        return ExitCode::from(1);
    }
    // Package shape: <name>.fang at the repo root; no nested dependencies.
    let entry = pkg_dir.join(format!("{}.fang", name));
    if !entry.exists() {
        let _ = std::fs::remove_dir_all(&pkg_dir);
        eprintln!(
            "fang add: '{}' has no {}.fang at its root; a package repo must contain {}",
            repo, name, name
        );
        return ExitCode::from(1);
    }
    if pkg_dir.join("fang_packages").exists() || pkg_dir.join("fang.lock").exists() {
        let _ = std::fs::remove_dir_all(&pkg_dir);
        eprintln!(
            "fang add: '{}' carries its own dependencies; transitive packages are not supported in v0.1",
            repo
        );
        return ExitCode::from(1);
    }
    let commit = Command::new("git")
        .current_dir(&pkg_dir)
        .args(["rev-parse", "HEAD"])
        .output();
    let commit = match commit {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    };
    // fang.lock: one "name commit" line per package.
    let mut lock = std::fs::read_to_string("fang.lock").unwrap_or_default();
    lock.push_str(&format!("{} {}\n", name, commit));
    std::fs::write("fang.lock", &lock).expect("write fang.lock");
    println!("installed {} at {}", name, &commit[..commit.len().min(12)]);
    println!("use it with: import {}", name);
    ExitCode::SUCCESS
}
