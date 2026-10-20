export interface Lesson {
  n: number
  title: string
  teach: string
  program: string
  expect: string
  hint: string
  recap: string
}

export const COURSE: Lesson[] = [
  {
    "n": 1,
    "title": "1. Making the computer talk",
    "teach": "A program is a file of lines, run top to bottom. The simplest\ninstruction is print: it shows text on the screen. Text goes between\ndouble quotes. No wrapper, no ceremony: the file IS the program.",
    "program": "print(\"___\")",
    "expect": "hello, fang\n",
    "hint": "Type hello, fang between the quotes. Keep the quotes.",
    "recap": "You wrote a program. One line, run top to bottom. Every longer\nprogram is just more lines."
  },
  {
    "n": 2,
    "title": "2. Naming values",
    "teach": "A variable is a name for a value. You create one with let. After that\nyou can use the name anywhere you would use the value, and you can\nchange it with =. f-strings (the f before the quote) let you put\nvalues inside text between braces.",
    "program": "name = \"___\"\nprint(f\"hi {name}\")",
    "expect": "hi fang\n",
    "hint": "Fill the blank with the word: fang (no quotes, it is already quoted).",
    "recap": "let creates the name once. The f-string dropped the value straight\ninto the sentence. Names are how you remember things as the program\nruns."
  },
  {
    "n": 3,
    "title": "3. Math",
    "teach": "Fang does arithmetic: + - * / and % (remainder after division).\nDivision of whole numbers drops the fraction: 7 / 2 is 3.\nWrite 7.0 / 2 for 3.5.",
    "program": "a = 6\nb = ___\nprint(a * b)",
    "expect": "42\n",
    "hint": "You need a number that makes 6 times it equal 42.",
    "recap": "Arithmetic works the way you expect. Whole-number division truncates; write 7.0 / 2 when you want the fraction."
  },
  {
    "n": 4,
    "title": "4. Making decisions",
    "teach": "Programs choose with if. The condition goes after if, a colon ends\nthe question, and the indented lines underneath run only when the\ncondition is true. else gives the other path.",
    "program": "age = ___\nif age >= 18:\n  print(\"adult\")\nelse:\n  print(\"minor\")",
    "expect": "adult\n",
    "hint": "Pick a number that is 18 or higher.",
    "recap": "if runs its block only when the condition holds. >= means greater-or-equal. Conditions are true or false, nothing else."
  },
  {
    "n": 5,
    "title": "5. Repeating work",
    "teach": "for repeats lines. `for i in 0..5:` runs its block with i set to\n0, 1, 2, 3, 4 in turn. The range 0..5 stops before 5. If you want to\ninclude the end, write 0..=5.",
    "program": "for i in 1..___:\n  print(f\"count {i}\")",
    "expect": "count 1\ncount 2\ncount 3\n",
    "hint": "The range stops right before the number. You want 1, 2, 3 printed.",
    "recap": "The loop ran the print three times, once per value. Loops plus decisions are most of programming."
  },
  {
    "n": 6,
    "title": "6. Lists: many values, one name",
    "teach": "A list holds several values in order. You build one with square\nbrackets, reach into it with [position], and len tells you how many\nit holds. Positions start at 0, and -1 means the last one.",
    "program": "fruits = [\"apple\", \"banana\", \"cherry\"]\nprint(fruits[___])",
    "expect": "banana\n",
    "hint": "Positions start counting at 0. Apple is position 0.",
    "recap": "fruits[0] is apple, fruits[1] is banana. len(fruits) is 3. Lists are the workhorse for collections of things."
  },
  {
    "n": 7,
    "title": "7. Teaching the computer new words",
    "teach": "A function is your own instruction: fn names it, the values it needs\ngo in parentheses. No types needed, and no return either: the last\nexpression IS the value it gives back.",
    "program": "fn double(n):\n  n * ___\n\nprint(double(21))",
    "expect": "42\n",
    "hint": "double should multiply by 2.",
    "recap": "You defined double and called it. The last line of a function is its value, just like a script. Functions let you name an idea once and reuse it everywhere."
  },
  {
    "n": 8,
    "title": "8. Talking to files",
    "teach": "Programs become real when they remember things. The fs module writes a file, reads it back, and checks whether it exists. write_file makes it, read_file hands you the text.",
    "program": "import fs\nfs.write_file(\"/tmp/leo-test/l8.txt\", \"___\")\nprint(fs.read_file(\"/tmp/leo-test/l8.txt\"))",
    "expect": "fang remembers\n",
    "hint": "Write the words: fang remembers (already inside the quotes).",
    "recap": "You wrote a real file to disk and read it back. Everything a program saves, loads, or shares goes through this door."
  },
  {
    "n": 9,
    "title": "9. Dictionaries",
    "teach": "A dictionary maps keys to values. Write one with curly braces, reach in with square brackets, and set adds or updates. keys(), values(), and items() hand you lists to loop over.",
    "program": "ages = {\"panda\": 30, \"fang\": 1}\nages[\"___\"] = 5\nprint(ages[\"panda\"], ages[\"nova\"])",
    "expect": "30 5\n",
    "hint": "The new key is nova.",
    "recap": "Dicts hold pairs. items() gives you both sides to loop: for k, v in ages.items()."
  },
  {
    "n": 10,
    "title": "10. Structs and methods",
    "teach": "A struct bundles named fields into one value. Create it by calling its name like a function. A method is a function whose first parameter is self: it gets the value before the dot.",
    "program": "struct Point:\n  x: float\n  y: float\n\nfn Point.hypot(self) -> float:\n  return (self.x * self.x + self.y * self.y) ** ___\n\np = Point(3.0, 4.0)\nprint(p.hypot())",
    "expect": "5\n",
    "hint": "hypot is the square root. 0.5 as the power means square root.",
    "recap": "p.hypot() called the method with p as self. Methods cost nothing over plain functions."
  },
  {
    "n": 11,
    "title": "11. Text is a toolbox",
    "teach": "Strings carry their own tools: upper, lower, strip, split, replace, find, starts_with. Dot-call them on any string.",
    "program": "s = \"  Hello, Fang  \"\nclean = s.strip()\nwords = clean.split(\", \")\nprint(words[___].upper())",
    "expect": "FANG\n",
    "hint": "split on comma-space gives [Hello, Fang]. You want the second word.",
    "recap": "strip trims, split cuts, upper shouts. Join glues them back."
  },
  {
    "n": 12,
    "title": "12. Building lists in one line",
    "teach": "A comprehension builds a list from another list: one expression, the loop, and an optional if filter.",
    "program": "xs = [1, 2, 3, 4, 5, 6]\nbig = [x * x for x in ___ if x > 3]\nprint(big)",
    "expect": "[16, 25, 36]\n",
    "hint": "The loop runs over the whole list; the if picks from it.",
    "recap": "That is the whole shape: [expr for x in list if cond]. Over dict pairs: for k, v in d.items()."
  },
  {
    "n": 13,
    "title": "13. Functions are values",
    "teach": "You can pass functions around like numbers. Write one without a name with fn(...): it is called a lambda. map, filter, and reduce take them.",
    "program": "xs = [1, 2, 3, 4]\ndoubles = xs.map(fn(x): return x * ___)\nprint(doubles)",
    "expect": "[2, 4, 6, 8]\n",
    "hint": "map replaces each element with the function's result.",
    "recap": "map transforms, filter keeps, reduce folds into one value. Named functions work too: xs.map(double)."
  },
  {
    "n": 14,
    "title": "14. When things go wrong",
    "teach": "raise throws an error. try runs code that might fail; except catches what was raised and names it e.",
    "program": "try:\n  raise \"___\"\nexcept e:\n  print(f\"caught: {e}\")",
    "expect": "caught: the disk is full\n",
    "hint": "Any text works. Try: the disk is full",
    "recap": "raise usually takes a string describing the problem. Uncaught errors stop the program with exit code 101."
  },
  {
    "n": 15,
    "title": "15. Splitting code into files",
    "teach": "Put functions in their own file and import them. import math uses the standard library; import utils looks for utils.fg next to your program.",
    "program": "import math\nprint(math.sqrt(___))",
    "expect": "3\n",
    "hint": "Fill: 9.0 - sqrt takes a float and 9.0 has root 3",
    "recap": "Modules keep their names: math.sqrt(9). Export what you want others to see with export fn."
  },
  {
    "n": 16,
    "title": "16. Calling C directly",
    "teach": "Any C function can be declared with extern and called like any other. This is how Fang reaches databases, GUIs, everything.",
    "program": "extern fn sqrt(x: float) -> float from \"math.h\"\nprint(sqrt(___))",
    "expect": "2\n",
    "hint": "What number has a square root of 2? Wait - pick a number whose root prints as 2.",
    "recap": "The types map straight across: int, float, bool, str, ptr. The header name goes in the from string."
  },
  {
    "n": 17,
    "title": "17. Closures: functions that remember",
    "teach": "A fn value keeps the values around it. In this lesson, capture happens when make_adder runs: the returned function remembers n.",
    "program": "fn make_adder(n):\n  return fn(x): return n + ___\n\nadd5 = make_adder(5)\nprint(add5(1))",
    "expect": "6\n",
    "hint": "add5 adds 5. What does the inner function add to x?",
    "recap": "The inner fn captured n. Run fang cheat for the whole language on one screen, and fang doc followed by any word for details."
  },
  {
    "n": 18,
    "title": "18. A real program: everything together",
    "teach": "Everything together: a guessing game. The program picks a secret,\nloops while the guess is wrong, and reacts. Lists, functions, if,\nand while in ten lines. Fill in what a correct guess prints.",
    "program": "secret = 7\nprint(\"guess a number between 1 and 10\")\nguesses = [3, 9, 7]\nfor guess in guesses:\n    if guess == secret:\n      print(\"___\")\n    elif guess < secret:\n      print(\"too low\")\n    else:\n      print(\"too high\")",
    "expect": "guess a number between 1 and 10\ntoo low\ntoo high\ncorrect!\n",
    "hint": "What should it print when the guess equals the secret? One word with an exclamation mark.",
    "recap": "That is a complete program: state (secret), a loop over data, and\ndecisions. You know enough now to read the full tutorial (LEARN.md)\nand write your own. Try: fang new myproject"
  }
]
