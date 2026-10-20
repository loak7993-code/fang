//! `fang learn`: an interactive terminal course that starts from zero
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
A program is a file of lines, run top to bottom. The simplest
instruction is print: it shows text on the screen. Text goes between
double quotes. No wrapper, no ceremony: the file IS the program.",
        program: "print(\"___\")",
        expect: "hello, fang\n",
        hint: "Type hello, fang between the quotes. Keep the quotes.",
        recap: "\
You wrote a program. One line, run top to bottom. Every longer
program is just more lines.",
    },
    Lesson {
        title: "2. Naming values",
        teach: "\
A variable is a name for a value. You create one with let. After that
you can use the name anywhere you would use the value, and you can
change it with =. f-strings (the f before the quote) let you put
values inside text between braces.",
        program: "\
name = \"___\"
print(f\"hi {name}\")",
        expect: "hi fang\n",
        hint: "Fill the blank with the word: fang (no quotes, it is already quoted).",
        recap: "\
let creates the name once. The f-string dropped the value straight
into the sentence. Names are how you remember things as the program
runs.",
    },
    Lesson {
        title: "3. Math",
        teach: "\
Fang does arithmetic: + - * / and % (remainder after division).
Division of whole numbers drops the fraction: 7 / 2 is 3.
Write 7.0 / 2 for 3.5.",
        program: "\
a = 6
b = ___
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
age = ___
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
fruits = [\"apple\", \"banana\", \"cherry\"]
print(fruits[___])",
        expect: "banana\n",
        hint: "Positions start counting at 0. Apple is position 0.",
        recap: "fruits[0] is apple, fruits[1] is banana. len(fruits) is 3. Lists are the workhorse for collections of things.",
    },
    Lesson {
        title: "7. Teaching the computer new words",
        teach: "\
A function is your own instruction: fn names it, the values it needs
go in parentheses. No types needed, and no return either: the last
expression IS the value it gives back.",
        program: "\
fn double(n):
  n * ___

print(double(21))",
        expect: "42\n",
        hint: "double should multiply by 2.",
        recap: "You defined double and called it. The last line of a function is its value, just like a script. Functions let you name an idea once and reuse it everywhere.",
    },
    Lesson {
        title: "9. Dictionaries",
        teach: "A dictionary maps keys to values. Write one with curly braces, reach in with square brackets, and set adds or updates. keys(), values(), and items() hand you lists to loop over.",
        program: "ages = {\"panda\": 30, \"fang\": 1}\nages[\"___\"] = 5\nprint(ages[\"panda\"], ages[\"nova\"])",
        expect: "30 5\n",
        hint: "The new key is nova.",
        recap: "Dicts hold pairs. items() gives you both sides to loop: for k, v in ages.items().",
    },
    Lesson {
        title: "10. Structs and methods",
        teach: "A struct bundles named fields into one value. Create it by calling its name like a function. A method is a function whose first parameter is self: it gets the value before the dot.",
        program: "struct Point:\n  x: float\n  y: float\n\nfn Point.hypot(self) -> float:\n  return (self.x * self.x + self.y * self.y) ** ___\n\np = Point(3.0, 4.0)\nprint(p.hypot())",
        expect: "5\n",
        hint: "hypot is the square root. 0.5 as the power means square root.",
        recap: "p.hypot() called the method with p as self. Methods cost nothing over plain functions.",
    },
    Lesson {
        title: "11. Text is a toolbox",
        teach: "Strings carry their own tools: upper, lower, strip, split, replace, find, starts_with. Dot-call them on any string.",
        program: "s = \"  Hello, Fang  \"\nclean = s.strip()\nwords = clean.split(\", \")\nprint(words[___].upper())",
        expect: "FANG\n",
        hint: "split on comma-space gives [Hello, Fang]. You want the second word.",
        recap: "strip trims, split cuts, upper shouts. Join glues them back.",
    },
    Lesson {
        title: "12. Building lists in one line",
        teach: "A comprehension builds a list from another list: one expression, the loop, and an optional if filter.",
        program: "xs = [1, 2, 3, 4, 5, 6]\nbig = [x * x for x in ___ if x > 3]\nprint(big)",
        expect: "[16, 25, 36]\n",
        hint: "The loop runs over the whole list; the if picks from it.",
        recap: "That is the whole shape: [expr for x in list if cond]. Over dict pairs: for k, v in d.items().",
    },
    Lesson {
        title: "13. Functions are values",
        teach: "You can pass functions around like numbers. Write one without a name with fn(...): it is called a lambda. map, filter, and reduce take them.",
        program: "xs = [1, 2, 3, 4]\ndoubles = xs.map(fn(x): return x * ___)\nprint(doubles)",
        expect: "[2, 4, 6, 8]\n",
        hint: "map replaces each element with the function's result.",
        recap: "map transforms, filter keeps, reduce folds into one value. Named functions work too: xs.map(double).",
    },
    Lesson {
        title: "14. When things go wrong",
        teach: "raise throws an error. try runs code that might fail; except catches what was raised and names it e.",
        program: "try:\n  raise \"___\"\nexcept e:\n  print(f\"caught: {e}\")",
        expect: "caught: the disk is full\n",
        hint: "Any text works. Try: the disk is full",
        recap: "raise usually takes a string describing the problem. Uncaught errors stop the program with exit code 101.",
    },
    Lesson {
        title: "15. Splitting code into files",
        teach: "Put functions in their own file and import them. import math uses the standard library; import utils looks for utils.fang next to your program.",
        program: "import math\nprint(math.sqrt(___))",
        expect: "3\n",
        hint: "Fill: 9.0 - sqrt takes a float and 9.0 has root 3",
        recap: "Modules keep their names: math.sqrt(9). Export what you want others to see with export fn.",
    },
    Lesson {
        title: "16. Calling C directly",
        teach: "Any C function can be declared with extern and called like any other. This is how Fang reaches databases, GUIs, everything.",
        program: "extern fn sqrt(x: float) -> float from \"math.h\"\nprint(sqrt(___))",
        expect: "2\n",
        hint: "What number has a square root of 2? Wait - pick a number whose root prints as 2.",
        recap: "The types map straight across: int, float, bool, str, ptr. The header name goes in the from string.",
    },
    Lesson {
        title: "17. Closures: functions that remember",
        teach: "A fn value keeps the values around it. In this lesson, capture happens when make_adder runs: the returned function remembers n.",
        program: "fn make_adder(n):\n  return fn(x): return n + ___\n\nadd5 = make_adder(5)\nprint(add5(1))",
        expect: "6\n",
        hint: "add5 adds 5. What does the inner function add to x?",
        recap: "The inner fn captured n. Run fang cheat for the whole language on one screen, and fang doc followed by any word for details.",
    },
    Lesson {
        title: "18. A real program: everything together",
        teach: "\
Everything together: a guessing game. The program picks a secret,
loops while the guess is wrong, and reacts. Lists, functions, if,
and while in ten lines. Fill in what a correct guess prints.",
        program: "\
secret = 7
print(\"guess a number between 1 and 10\")
guesses = [3, 9, 7]
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
and write your own. Try: fang new myproject",
    },
];

fn progress_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".fang").join("learn-progress")
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
    println!("fang learn - a programming course from zero");
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
            let dir = std::env::temp_dir().join("fang-learn");
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join(format!("lesson{}.fang", idx + 1));
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
        println!("  fang new myproject   (starter project)");
        println!("  LEARN.md            (the full tutorial)");
        println!("  examples/           (runnable programs)");
    }
    ExitCode::SUCCESS
}

pub fn cmd_new(args: &[String]) -> ExitCode {
    let name = match args.first() {
        Some(n) => n.clone(),
        None => {
            eprintln!("fang new: usage: fang new <name>");
            return ExitCode::from(1);
        }
    };
    let dir = std::path::Path::new(&name);
    if dir.exists() {
        eprintln!("fang new: '{}' already exists", name);
        return ExitCode::from(1);
    }
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("fang new: cannot create '{}': {}", name, e);
        return ExitCode::from(1);
    }
    let main = format!(
        "# {} - your first Fang program\nprint(\"hello from {}\")\n",
        name, name
    );
    if let Err(e) = std::fs::write(dir.join("main.fang"), main) {
        eprintln!("fang new: cannot write main.fang: {}", e);
        return ExitCode::from(1);
    }
    println!("created {}/main.fang", name);
    println!("run it with:  fang run {}/main.fang", name);
    ExitCode::SUCCESS
}
