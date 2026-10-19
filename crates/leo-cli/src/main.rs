//! The `leo` binary.

use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const USAGE: &str = "\
leo - looks like Python, runs like C

USAGE:
    leo <COMMAND> [ARGS]

COMMANDS:
    run <file.leo>        compile and execute a Leo program
    build <file.leo>      produce a standalone native binary
    repl                  interactive session
    fmt <files...>        format source files in the canonical style
    test [dir]            discover and run *_test.leo files
    add <repo>            install a package from a git repository
    version               print the version
    help                  print this help

EXIT CODES:
    0    success
    1    compile error or CLI misuse
    2    test failure
    101  runtime panic in a Leo program
";

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
        "fmt" => cmd_fmt(&args[1..]),
        cmd => {
            eprintln!("leo: unknown or unfinished command '{}'", cmd);
            eprint!("{}", USAGE);
            ExitCode::from(1)
        }
    }
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
        match leo_core::parse_source("fmt", &src) {
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
