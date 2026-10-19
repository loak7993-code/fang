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
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    match cmd {
        "version" | "--version" | "-V" => {
            println!("leo {}", VERSION);
            ExitCode::SUCCESS
        }
        "help" | "--help" | "-h" => {
            print!("{}", USAGE);
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("leo: unknown or unfinished command '{}'", cmd);
            eprint!("{}", USAGE);
            ExitCode::from(1)
        }
    }
}
