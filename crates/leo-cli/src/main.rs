//! The `leo` binary: run, build, repl, fmt, test, add.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use leo_core::{parse_source, sema, Analysis, GenOpts, Module, SourceUnit};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const USAGE: &str = "\
leo - looks like Python, runs like C

USAGE:
    leo <COMMAND> [ARGS]

COMMANDS:
    run <file.leo> [--release] [--keep-c] [--freestanding] [--target TRIPLE]
                          compile and execute a Leo program
    build <file.leo> [--release] [-o name] [--freestanding] [--target TRIPLE]
                          produce a standalone native binary
    repl                  interactive session
    fmt <files...>        format source files in the canonical style
    test [dir]            discover and run *_test.leo files
    add <repo>            install a package from a git repository
    version               print the version
    help                  print this help

The toolchain needs cc (gcc or clang) on PATH.

EXIT CODES:
    0    success
    1    compile error or CLI misuse
    2    test failure
    101  runtime panic in a Leo program
";

/// The C runtime, embedded so the leo binary is self-contained.
const RT_H: &str = include_str!("../../../runtime/leo_rt.h");
const RT_C: &str = include_str!("../../../runtime/leo_rt.c");

/// Standard library modules, embedded. Populated as std/*.leo are written.
static STD_MODULES: &[(&str, &str)] = &[];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("help") {
        "version" | "--version" | "-V" => {
            println!("leo {}", VERSION);
            ExitCode::SUCCESS
        }
        "help" | "--help" | "-h" => {
            print!("{}", USAGE);
            ExitCode::SUCCESS
        }
        "run" => cmd_run(&args[1..]),
        "build" => cmd_build(&args[1..]),
        "fmt" => cmd_fmt(&args[1..]),
        cmd => {
            eprintln!("leo: unknown or unfinished command '{}'", cmd);
            eprint!("{}", USAGE);
            ExitCode::from(1)
        }
    }
}

struct BuildFlags {
    release: bool,
    keep_c: bool,
    freestanding: bool,
    target: Option<String>,
    output: Option<String>,
}

fn parse_flags(args: &[String]) -> Result<(String, BuildFlags), String> {
    let mut file: Option<String> = None;
    let mut flags = BuildFlags {
        release: false,
        keep_c: false,
        freestanding: false,
        target: None,
        output: None,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--release" => flags.release = true,
            "--keep-c" => flags.keep_c = true,
            "--freestanding" => flags.freestanding = true,
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
    let file = file.ok_or("pass a .leo file")?;
    if !file.ends_with(".leo") {
        return Err(format!("'{}' is not a .leo file", file));
    }
    Ok((file, flags))
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
    let module = parse_source(&name, &src)?;
    let mut units = Vec::new();
    let mut seen: Vec<String> = Vec::new();
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
        if let leo_core::ast::Item::Import { module: imp, path: ipath, .. } = item {
            if seen.iter().any(|s| s == imp) {
                continue; // already loaded
            }
            // 1. embedded std module
            if let Some((_, src)) = STD_MODULES.iter().find(|(n, _)| n == imp) {
                let m = parse_source(imp, src)?;
                load_imports(m, imp.clone(), format!("<std>/{}.leo", imp), units, seen, base_dir)?;
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
            // 3. sibling file <name>.leo
            let sibling = base_dir.parent().unwrap_or(Path::new(".")).join(format!("{}.leo", imp));
            // 4. package
            let pkg = base_dir
                .parent()
                .unwrap_or(Path::new("."))
                .join("leo_packages")
                .join(imp)
                .join(format!("{}.leo", imp));
            let resolved = if sibling.exists() {
                sibling
            } else if pkg.exists() {
                pkg
            } else {
                return Err(format!(
                    "unknown module '{}'; not a std module, a local {}.leo, or a package",
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

fn analyze_program(units: &[SourceUnit], entry: &str) -> Result<Analysis, String> {
    let sources: Vec<(String, Module)> = units
        .iter()
        .map(|u| (u.name.clone(), u.module.clone()))
        .collect();
    let a = sema::analyze(sources, entry);
    if a.errors.is_empty() {
        return Ok(a);
    }
    // Report up to 10 errors, caret formatted.
    for e in a.errors.iter().take(10) {
        eprintln!("{}:{}:{}: error: {}", e.module, e.line, e.col, e.msg);
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
    let c_src = leo_core::codegen::generate(units, a, entry, &opts)?;

    let build_dir = std::env::temp_dir().join(format!("leo-build-{}", tag));
    std::fs::create_dir_all(&build_dir).map_err(|e| format!("cannot create build dir: {}", e))?;
    let c_path = build_dir.join("program.c");
    std::fs::write(&c_path, &c_src).map_err(|e| format!("cannot write C: {}", e))?;
    std::fs::write(build_dir.join("leo_rt.h"), RT_H).map_err(|e| e.to_string())?;
    std::fs::write(build_dir.join("leo_rt.c"), RT_C).map_err(|e| e.to_string())?;

    if flags.keep_c {
        if let Some(dir) = Path::new(&units.last().unwrap().path).parent() {
            let _ = std::fs::write(dir.join("program.leo.c"), &c_src);
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

    let opt = if flags.release { "-O2" } else { "-O0" };
    let mut cmd = Command::new("cc");
    cmd.arg(opt)
        .arg("-std=c11")
        .arg("-I")
        .arg(&build_dir)
        .arg(&c_path);
    if !flags.freestanding {
        cmd.arg(build_dir.join("leo_rt.c"));
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
            eprintln!("leo run: {}", e);
            return ExitCode::from(1);
        }
    };
    match run_program(&file, &flags, "run") {
        Ok(code) => code,
        Err(e) => {
            eprintln!("leo: {}", e);
            ExitCode::from(1)
        }
    }
}

fn cmd_build(args: &[String]) -> ExitCode {
    let (file, flags) = match parse_flags(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("leo build: {}", e);
            return ExitCode::from(1);
        }
    };
    match build_program(&file, &flags) {
        Ok(path) => {
            println!("built: {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("leo: {}", e);
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
    // Move the binary next to the source file.
    let out = Path::new(file).with_file_name(
        flags.output.clone().unwrap_or_else(|| {
            format!(
                "{}{}",
                Path::new(file)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("program"),
                ""
            )
        }),
    );
    std::fs::copy(&compiled.binary, &out).map_err(|e| format!("cannot write output: {}", e))?;
    Ok(out)
}

fn cmd_fmt(files: &[String]) -> ExitCode {
    if files.is_empty() {
        eprintln!("leo fmt: pass at least one .leo file");
        return ExitCode::from(1);
    }
    let mut failed = false;
    for path in files {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("leo fmt: cannot read {}: {}", path, e);
                failed = true;
                continue;
            }
        };
        match parse_source("fmt", &src) {
            Ok(m) => {
                let out = leo_core::format_module(&m);
                if out != src {
                    if let Err(e) = std::fs::write(path, &out) {
                        eprintln!("leo fmt: cannot write {}: {}", path, e);
                        failed = true;
                        continue;
                    }
                    println!("formatted: {}", path);
                } else {
                    println!("unchanged: {}", path);
                }
            }
            Err(e) => {
                eprintln!("leo fmt: {}: {}", path, e);
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
