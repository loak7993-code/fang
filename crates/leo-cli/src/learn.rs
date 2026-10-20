//! `leo learn`: an interactive terminal course that starts from zero
//! programming experience. Each lesson explains a concept in plain
//! language, then has the learner fill in a blank in a real program,
//! compiles it, runs it, and checks the output.

use std::process::{Command, ExitCode};

struct Lesson {
    title: &'static str,
    /// Plain-language explanation, printed before the challenge.
    teach: &'static str,
    /// Program with `___` where the learner fills in.
    program: &'static str,
    /// Exact stdout the completed program must produce.
    expect: &'static str,
    /// Shown after a wrong answer.
    hint: &'static str,
    /// Shown after success, explaining what they just did.
    recap: &'static str,
}

const LESSONS: &[Lesson] = &[
    Lesson {
        title: "1. Making the computer talk",
        teach: "\
Every program is a list of instructions. The simplest instruction is
print: it shows text on the screen. Text in Leo is written between
double quotes. Every program starts from a function called main, and
the lines inside it run one after another.",
        program: "\
fn main():
  print(\"___\")",
        expect: "hello, leo\n",
        hint: "Type hello, leo between the quotes. Keep the quotes.",
        recap: "\
You wrote a program. The fn main(): line says where the program
starts, the two spaces say \"this line belongs to main\", and print
showed your text. That structure carries everything else you learn.",
    },
    Lesson {
        title: "2. Naming values",
        teach: "\
A variable is a name for a value. You create one with let. After that
you can use the name anywhere you would use the value, and you can
change it with =. f-strings (the f before the quote) let you put
values inside text between braces.",
        program: "\
fn main():
  let name = \"___\"
  print(f\"hi {name}\")",
        expect: "hi leo\n",
        hint: "Fill the blank with the word: leo (no quotes, it is already quoted).",
        recap: "\
let creates the name once. The f-string dropped the value straight
into the sentence. Names are how you remember things as the program
runs.",
    },
    Lesson {
        title: "3. Math",
        teach: "\
Leo does arithmetic: + - * / and % (remainder after division).
Division of whole numbers drops the fraction: 7 / 2 is 3.
Write 7.0 / 2 for 3.5.",
        program: "\
fn main():
  let a = 6
  let b = ___
  print(a * b)",
        expect: "42\n",
        hint: "You need a number that makes 6 times it equal 42.",
        recap: "Arithmetic works the way you expect. Whole-number division truncates; write 7.0 / 2 when you want the fraction.",
    },
    Lesson {
        title: "4. Making decisions",
        teach: "\
Programs choose with if. The condition goes after if, a colon ends
the question, and the indented lines underneath run only when the
condition is true. else gives the other path.",
        program: "\
fn main():
  let age = ___
  if age >= 18:
    print(\"adult\")
  else:
    print(\"minor\")",
        expect: "adult\n",
        hint: "Pick a number that is 18 or higher.",
        recap: "if runs its block only when the condition holds. >= means greater-or-equal. Conditions are true or false, nothing else.",
    },
    Lesson {
        title: "5. Repeating work",
        teach: "\
for repeats lines. `for i in 0..5:` runs its block with i set to
0, 1, 2, 3, 4 in turn. The range 0..5 stops before 5. If you want to
include the end, write 0..=5.",
        program: "\
fn main():
  for i in 1..___:
    print(f\"count {i}\")",
        expect: "count 1\ncount 2\ncount 3\n",
        hint: "The range stops right before the number. You want 1, 2, 3 printed.",
        recap: "The loop ran the print three times, once per value. Loops plus decisions are most of programming.",
    },
    Lesson {
        title: "6. Lists: many values, one name",
        teach: "\
A list holds several values in order. You build one with square
brackets, reach into it with [position], and len tells you how many
it holds. Positions start at 0, and -1 means the last one.",
        program: "\
fn main():
  let fruits = [\"apple\", \"banana\", \"cherry\"]
  print(fruits[___])",
        expect: "banana\n",
        hint: "Positions start counting at 0. Apple is position 0.",
        recap: "fruits[0] is apple, fruits[1] is banana. len(fruits) is 3. Lists are the workhorse for collections of things.",
    },
    Lesson {
        title: "7. Teaching the computer new words",
        teach: "\
A function is your own instruction. fn names it, the values it needs
go in parentheses with their types, and return hands a value back.
You then call it by name anywhere.",
        program: "\
fn double(n: int) -> int:
  return n * ___

fn main():
  print(double(21))",
        expect: "42\n",
        hint: "double should multiply by 2.",
        recap: "You defined double and called it. The -> int says what it gives back. Functions let you name an idea once and reuse it everywhere.",
    },
    Lesson {
        title: "8. A real program",
        teach: "\
Everything together: a guessing game. The program picks a secret,
loops while the guess is wrong, and reacts. Lists, functions, if,
and while in ten lines. Fill in what a correct guess prints.",
        program: "\
fn main():
  let secret = 7
  print(\"guess a number between 1 and 10\")
  let guesses = [3, 9, 7]
  for guess in guesses:
    if guess == secret:
      print(\"___\")
    elif guess < secret:
      print(\"too low\")
    else:
      print(\"too high\")",
        expect: "\
guess a number between 1 and 10
too low
too high
correct!\n",
        hint: "What should it print when the guess equals the secret? One word with an exclamation mark.",
        recap: "\
That is a complete program: state (secret), a loop over data, and
decisions. You know enough now to read the full tutorial (LEARN.md)
and write your own. Try: leo new myproject",
    },
];

fn progress_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".leo").join("learn-progress")
}

fn load_progress() -> usize {
    std::fs::read_to_string(progress_file())
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn save_progress(n: usize) {
    let p = progress_file();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(p, n.to_string());
}

pub fn cmd_learn(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "list" => {
            for (i, l) in LESSONS.iter().enumerate() {
                let done = if i < load_progress() { "done" } else { "    " };
                println!("{}  {}", done, l.title);
            }
            return ExitCode::SUCCESS;
        }
        "reset" => {
            save_progress(0);
            println!("progress cleared");
            return ExitCode::SUCCESS;
        }
        _ => {}
    }

    let mut idx = load_progress();
    let mut rl = rustyline::DefaultEditor::new().expect("terminal");
    println!("leo learn - a programming course from zero");
    println!("{} lessons. ctrl-c to leave and resume later.", LESSONS.len());
    println!();

    'lessons: while idx < LESSONS.len() {
        let lesson = &LESSONS[idx];
        println!("=== {} ===", lesson.title);
        println!("{}", lesson.teach);
        println!("Here is the program. ___ is your blank to fill:");
        println!();
        for line in lesson.program.lines() {
            println!("    {}", line);
        }
        println!();

        let mut attempts = 0;
        loop {
            println!("Fill the blank (or: hint, skip, quit):");
            let line = match rl.readline("> ") {
                Ok(l) => l,
                Err(_) => break 'lessons,
            };
            let ans = line.trim();
            if ans.is_empty() {
                continue;
            }
            match ans {
                "quit" | "q" => break 'lessons,
                "skip" => {
                    println!("skipping. {}", lesson.recap);
                    break;
                }
                "hint" | "h" => {
                    println!("hint: {}", lesson.hint);
                    continue;
                }
                _ => {}
            }
            let filled = lesson.program.replacen("___", ans, 1);
            let dir = std::env::temp_dir().join("leo-learn");
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join(format!("lesson{}.leo", idx + 1));
            if std::fs::write(&file, &filled).is_err() {
                eprintln!("cannot write the lesson file");
                return ExitCode::from(1);
            }
            let out = Command::new(std::env::current_exe().unwrap())
                .arg("run")
                .arg(&file)
                .output();
            let out = match out {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("cannot run the program: {}", e);
                    return ExitCode::from(1);
                }
            };
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() && stdout == lesson.expect {
                println!();
                println!("correct! output:");
                print!("{}", stdout);
                println!("{}", lesson.recap);
                println!();
                break;
            }
            attempts += 1;
            if !stderr.trim().is_empty() {
                println!();
                println!("it did not compile:");
                for line in stderr.lines().take(4) {
                    println!("  {}", line);
                }
            } else {
                println!();
                println!("it ran, but the output was:");
                for line in stdout.lines().take(4) {
                    println!("  {}", line);
                }
            }
            if attempts >= 2 {
                println!("hint: {}", lesson.hint);
            }
            println!("try again:");
        }
        idx += 1;
        save_progress(idx);
    }

    if idx >= LESSONS.len() {
        println!("course complete. you know: values, names, math, decisions,");
        println!("loops, lists, and functions. next steps:");
        println!("  leo new myproject   (starter project)");
        println!("  LEARN.md            (the full tutorial)");
        println!("  examples/           (runnable programs)");
    }
    ExitCode::SUCCESS
}

pub fn cmd_new(args: &[String]) -> ExitCode {
    let name = match args.first() {
        Some(n) => n.clone(),
        None => {
            eprintln!("leo new: usage: leo new <name>");
            return ExitCode::from(1);
        }
    };
    let dir = std::path::Path::new(&name);
    if dir.exists() {
        eprintln!("leo new: '{}' already exists", name);
        return ExitCode::from(1);
    }
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("leo new: cannot create '{}': {}", name, e);
        return ExitCode::from(1);
    }
    let main = format!(
        "# {} - your first Leo program\nfn main():\n  print(\"hello from {}\")\n",
        name, name
    );
    if let Err(e) = std::fs::write(dir.join("main.leo"), main) {
        eprintln!("leo new: cannot write main.leo: {}", e);
        return ExitCode::from(1);
    }
    println!("created {}/main.leo", name);
    println!("run it with:  leo run {}/main.leo", name);
    ExitCode::SUCCESS
}
