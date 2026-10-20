import { useState } from 'react'
import { Lightbulb, ArrowLeft, ArrowRight } from '@phosphor-icons/react'
import { COURSE } from './course'
import { BENCH, BUILD_STATS } from './bench'

type View = 'home' | 'learn' | 'bench' | 'cheat' | 'install'

const GITHUB = 'https://github.com/loak7993-code/fang'

/* ---------------- shared ---------------- */

function CodeBlock({
  name,
  code,
  output,
  showLines = false,
}: {
  name: string
  code: string
  output?: string
  showLines?: boolean
}) {
  const [copied, setCopied] = useState(false)
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(true)
      setTimeout(() => setCopied(false), 1400)
    } catch {
      setCopied(false)
    }
  }
  const parts = code.split('___')
  const lines = code.split('\n')
  return (
    <div className="code">
      <div className="code-head">
        <span className="code-name">{name}</span>
        <button
          className={'copy-btn' + (copied ? ' copied' : '')}
          onClick={copy}
          aria-label="Copy code"
        >
          {copied ? 'copied' : 'copy'}
        </button>
      </div>
      <pre>
        {showLines && (
          <span className="cm">
            {lines.map((_, i) => String(i + 1).padStart(2, ' ') + '  ').join('\n')}
            {'\n'}
          </span>
        )}
        {parts.map((part, i) => (
          <span key={i}>
            {part}
            {i < parts.length - 1 && <span className="blank">___</span>}
          </span>
        ))}
      </pre>
      {output !== undefined && <div className="output">{output.replace(/\n$/, '')}</div>}
    </div>
  )
}

function TopBar({ view, go }: { view: View; go: (v: View) => void }) {
  const links: [View, string][] = [
    ['home', 'fang'],
    ['learn', 'learn'],
    ['bench', 'measured'],
    ['cheat', 'cheat'],
    ['install', 'install'],
  ]
  return (
    <header className="topbar">
      <div className="topbar-inner">
        <a
          className="wordmark"
          href="#/"
          onClick={(e) => {
            e.preventDefault()
            go('home')
          }}
        >
          fang
        </a>
        <nav>
          {links
            .filter(([v]) => v !== 'home')
            .map(([v, label]) => (
              <a
                key={v}
                href={'#/' + v}
                className={
                  'toplink' +
                  (view === v ? ' active' : '') +
                  (v === 'cheat' || v === 'install' ? ' hide-m' : '')
                }
                onClick={(e) => {
                  e.preventDefault()
                  go(v)
                }}
              >
                {label}
              </a>
            ))}
          <a className="toplink-gh" href={GITHUB} target="_blank" rel="noreferrer">
            github
          </a>
        </nav>
      </div>
    </header>
  )
}

/* ---------------- home ---------------- */

const BUILD_ROWS: [string, string, string][] = [
  [
    'scripts',
    'scripts and CLI tools',
    'Plain lines in a file, run top to bottom. No fn main, no return keyword, types inferred from use.',
  ],
  [
    'servers',
    'backend websites',
    'The HTTP server is pure Fang. Handlers are plain function values; GET and POST verified end to end with curl.',
  ],
  [
    'tui',
    'terminal apps',
    'Screen buffers, colors, raw key input, zero dependencies. The CHIP-8 emulator built on it runs at 33M instructions per second.',
  ],
  [
    'kernels',
    'kernels and bare metal',
    'Freestanding builds emit C with no runtime and no libc, passing any target triple to the platform compiler.',
  ],
  [
    'python',
    'python libraries',
    'import py bridges CPython when you want numpy or the ecosystem, opt-in at link time.',
  ],
  [
    'os',
    'every operating system',
    'C reaches Linux, Windows, macOS, the BSDs, Android and iOS. Cross-compile by target triple.',
  ],
]

function Home({ go }: { go: (v: View) => void }) {
  return (
    <>
      <section className="hero">
        <div className="hero-left">
          <div className="hero-eyebrow rise">
            <span className="dot">_</span> a language that compiles to C
          </div>
          <h1 className="rise rise-1">
            python's
            <br />
            <span className="thin">flexibility.</span>
            <br />
            c's speed<span style={{ color: 'var(--accent)' }}>.</span>
          </h1>
          <p className="hero-sub rise rise-2">
            Fang is a programming language that compiles through C to{' '}
            <b>native code</b>. Scripts with zero ceremony. Servers, terminal
            apps, emulators, kernels. One language, native everywhere C
            ships.
          </p>
          <div className="hero-ctas rise rise-3">
            <button className="btn btn-solid" onClick={() => go('learn')}>
              learn the language
            </button>
            <button className="btn btn-line" onClick={() => go('install')}>
              install
            </button>
          </div>
        </div>
        <div className="term">
          <div className="term-bar">
            <span className="term-dot" />
            <span className="term-dot" />
            <span className="term-dot live" />
            <span className="term-title">fang run hello.fg</span>
          </div>
          <pre>
            <span className="prompt">$ </span>cat hello.fg
            {'\n'}print(<span style={{ color: 'var(--accent)' }}>"hello, fang"</span>)
            {'\n'}
            <span className="prompt">$ </span>fang run hello.fg
            {'n'}
            <span className="out">hello, fang</span>
            {'\n\n'}
            <span className="prompt">$ </span>fang build hello.fg --release
            {'\n'}built: hello.bin <span className="cm">/* 91ms, static */</span>
            {'\n\n'}
            <span className="prompt">$ </span>
            <span className="cursor" />
          </pre>
        </div>
      </section>

      <div className="strip">
        <div className="strip-inner">
          {BUILD_STATS.map((s) => (
            <div className="strip-cell" key={s.label}>
              <div className="strip-num">
                {s.num}
                <span className="u">{s.unit}</span>
              </div>
              <div className="strip-label">{s.label}</div>
            </div>
          ))}
        </div>
      </div>

      <section className="sect">
        <div className="sect-head">
          <span className="sect-no">§ 01</span>
          <h2 className="sect-title">the language in one minute</h2>
        </div>
        <div className="split">
          <CodeBlock
            name="tour.fg"
            code={`# scripts are just lines
name = "world"
print(f"hello {name}")

# last expression is the return value
fn double(n):
  n * 2

# module-level let is a mutable global
let hits: list[int] = []

fn track(n):
  hits.push(n)
  len(hits)

track(1)
track(2)
print(track(3))

# comprehensions, closures, structs
evens = [x * 2 for x in 0..5]
add = fn(a, b): a + b
print(evens, add(3, 4))

struct Point:
  x: float
  y: float

fn Point.hypot(self):
  (self.x ** 2 + self.y ** 2) ** 0.5

p = Point(3.0, 4.0)
print(p.hypot())`}
            output={`hello world
3
[0, 2, 4, 6, 8] 7
5`}
          />
          <div className="commentary">
            <p>
              <b>No ceremony.</b> No fn main, no return keyword, no type
              declarations. The file is the program; the last expression is
              the value.
            </p>
            <p>
              <b>Module-level state works like Python.</b> Top-level let
              creates a mutable global every function in the file can see
              and change.
            </p>
            <p>
              <b>Everything is a value.</b> Comprehensions, lambdas,
              function types as annotations: <span className="k">{'fn(Request) -> Response'}</span>.
            </p>
            <p>
              <b>Methods attach to structs</b> with a plain function and a{' '}
              <span className="k">self</span> parameter. Bitwise operators,
              hex and binary literals included.
            </p>
          </div>
        </div>
      </section>

      <section className="sect">
        <div className="sect-head">
          <span className="sect-no">§ 02</span>
          <h2 className="sect-title">what you can build</h2>
        </div>
        <div className="rows">
          {BUILD_ROWS.map(([id, title, text], i) => (
            <div className="row" key={id}>
              <span className="idx">{String(i + 1).padStart(2, '0')}</span>
              <h3>{title}</h3>
              <p>{text}</p>
              <span className="go">{'->'}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="sect">
        <div className="sect-head">
          <span className="sect-no">§ 03</span>
          <h2 className="sect-title">the whole course</h2>
        </div>
        <p className="sect-note">
          18 lessons, one real program per lesson, a blank to fill. In your
          terminal it is <b>fang learn</b>; here it is the same course,
          same programs, same expected output.
        </p>
        <div className="split">
          <CodeBlock
            name="lesson1.fg"
            code={`print("hello, fang")`}
            output={'hello, fang'}
          />
          <div className="commentary">
            <p>
              <b>Lesson one is one line.</b> The course teaches the
              zero-ceremony path first: plain script lines, functions at
              lesson 7 without types or returns.
            </p>
            <p>
              <b>By lesson 18</b> you write a guessing game with loops,
              branches, lists and input.
            </p>
            <button
              className="btn btn-solid"
              style={{ marginTop: 16 }}
              onClick={() => go('learn')}
            >
              start lesson 01
            </button>
          </div>
        </div>
      </section>
    </>
  )
}

/* ---------------- learn ---------------- */

function Learn() {
  const [idx, setIdx] = useState(0)
  const lesson = COURSE[idx]
  const short = (t: string) => t.split('. ')[1] || t
  return (
    <div className="sect" style={{ paddingBottom: 40 }}>
      <div className="sect-head">
        <span className="sect-no">§ 04</span>
        <h2 className="sect-title">learn</h2>
      </div>
      <div className="learn-layout">
        <div className="lesson-index">
          {COURSE.map((l, i) => (
            <button
              key={l.n}
              className={i === idx ? 'active' : ''}
              onClick={() => setIdx(i)}
            >
              <span className="n">{String(l.n).padStart(2, '0')}</span>
              <span>{short(l.title)}</span>
            </button>
          ))}
        </div>
        <div className="lesson" key={idx}>
          <div className="lesson-kicker">
            lesson <span style={{ color: 'var(--accent)' }}>{String(lesson.n).padStart(2, '0')}</span>{' '}
            <span className="of">/ 18</span>
          </div>
          <h2>{short(lesson.title)}</h2>
          <p className="teach">{lesson.teach}</p>
          <div className="stack">
            <CodeBlock name={'lesson' + lesson.n + '.fg'} code={lesson.program} showLines />
            <CodeBlock name="expected output" code={lesson.expect.trimEnd()} />
          </div>
          <details className="hint-box">
            <summary>
              <Lightbulb weight="bold" size={13} /> show the hint
            </summary>
            <p style={{ marginTop: 10 }}>{lesson.hint}</p>
          </details>
          <div className="recap">{lesson.recap}</div>
          <div className="lesson-nav">
            <button className="pager" disabled={idx === 0} onClick={() => setIdx(idx - 1)}>
              <ArrowLeft weight="bold" size={13} /> prev
            </button>
            <button
              className="pager"
              disabled={idx === COURSE.length - 1}
              onClick={() => setIdx(idx + 1)}
            >
              next <ArrowRight weight="bold" size={13} />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

/* ---------------- bench ---------------- */

function Bench() {
  return (
    <div className="sect">
      <div className="sect-head">
        <span className="sect-no">§ 05</span>
        <h2 className="sect-title">measured</h2>
      </div>
      <p className="sect-note">
        Median of 5 runs, seconds, lower is better. Fang builds at{' '}
        <b>-O3 -march=native</b>, same as the C and Rust baselines. The
        harness lives in the repository, the numbers are from real runs.
      </p>
      {BENCH.map((b) => {
        const rows: [string, number | null, boolean][] = [
          ['fang', b.fang, true],
          ['c', b.c, false],
          ['rust', b.rust, false],
          ['c++', b.cpp, false],
          ['python', b.python, false],
        ]
        const valid = rows.filter((r) => r[1] !== null) as [string, number, boolean][]
        const max = Math.max(...valid.map((r) => r[1]))
        return (
          <div className="bench-block" key={b.name}>
            <div>
              <div className="bench-name">{b.name}</div>
              <div className="bench-verdict">{b.verdict}</div>
            </div>
            <div className="bench-rows">
              {valid.map(([lang, t, isFang]) => (
                <div className={'bar-row' + (isFang ? ' fang' : '')} key={lang}>
                  <span className="lang">{lang}</span>
                  <div className="bar-track">
                    <div
                      className="bar-fill"
                      style={{
                        width: `${(t / max) * 100}%`,
                        animationDelay: `${valid.indexOf(
                          valid.find((r) => r[0] === lang) ?? valid[0],
                        ) * 0.06}s`,
                      }}
                    />
                  </div>
                  <span className="val">{t.toFixed(3)}s</span>
                </div>
              ))}
            </div>
          </div>
        )
      })}

      <div className="sect-head" style={{ marginTop: 90 }}>
        <span className="sect-no">§ 06</span>
        <h2 className="sect-title">toolchain</h2>
      </div>
      <p className="sect-note">
        Incremental compilation: every module compiles once and is cached
        as an object file. Only your entry file compiles on every build.
      </p>
      <div className="tool-strip">
        <div className="tool-cell">
          <div className="tool-num">
            91<span className="u">ms</span>
          </div>
          <p>release build, server + json program</p>
        </div>
        <div className="tool-cell">
          <div className="tool-num">
            108<span className="u">ms</span>
          </div>
          <p>rustc -O, same size program</p>
        </div>
        <div className="tool-cell">
          <div className="tool-num">
            61<span className="u">ms</span>
          </div>
          <p>cold start, compile and execute</p>
        </div>
        <div className="tool-cell">
          <div className="tool-num">
            50<span className="u">kb</span>
          </div>
          <p>static hello world binary</p>
        </div>
      </div>
    </div>
  )
}

/* ---------------- cheat ---------------- */

const CHEAT: [string, string][] = [
  ['VARIABLES', `x = 5              # plain assignment
let hits: list[int] = []   # module-level global
name = "fang"`],
  ['CONTROL FLOW', `if x > 10:
  "big"
elif x > 5:
  "medium"
else:
  "small"

for i in 0..10:      # exclusive range
  print(i)
for i in 1..=10:     # inclusive
  print(i)
while x > 0:
  x = x - 1`],
  ['FUNCTIONS', `fn double(n):
  n * 2              # last expression returns

fn fib(n: int) -> int:   # recursion declares the type
  if n < 2:
    return n
  fib(n - 1) + fib(n - 2)

fn Point.hypot(self):   # method
  (self.x ** 2 + self.y ** 2) ** 0.5

add = fn(a, b): a + b   # lambda value`],
  ['COLLECTIONS', `xs = [1, 2, 3]
xs.push(4)
evens = [x * 2 for x in xs if x % 2 == 0]

ages = {"panda": 30, "fang": 1}
ages["nova"] = 5
for k, v in ages.items():
  print(k, v)`],
  ['STRUCTS AND ERRORS', `struct Point:
  x: float
  y: float

try:
  risky()
except err:
  print("caught", err)`],
  ['BITS AND LITERALS', `mask = 0xF0 | 0x0F
low = byte & 15
wide = 1 << 8
bits = 0b1010`],
  ['MODULES', `import server
import str

fn handler(req: Request) -> Response:
  if req.path == "/":
    return server.html("<h1>hi</h1>")
  return server.not_found()`],
  ['TUI', `import tui

tui.init()
s = tui.screen(40, 12)
tui.scr_set_str(s, 2, 2, "hello")
k = tui.read_key(30)
if k == tui.KEY_LEFT:
  print("left")
tui.done()`],
]

function Cheat() {
  return (
    <div className="sect">
      <div className="sect-head">
        <span className="sect-no">§ 07</span>
        <h2 className="sect-title">cheat sheet</h2>
      </div>
      <p className="sect-note">
        The whole language fits on one screen. Same content ships in the
        compiler as <b>fang cheat</b>.
      </p>
      <div className="cheat-grid">
        {CHEAT.map(([name, code]) => (
          <div className="cheat-block" key={name}>
            <h3>{name}</h3>
            <pre>{code}</pre>
          </div>
        ))}
      </div>
    </div>
  )
}

/* ---------------- install ---------------- */

function Install() {
  return (
    <div className="sect">
      <div className="sect-head">
        <span className="sect-no">§ 08</span>
        <h2 className="sect-title">install</h2>
      </div>
      <p className="sect-note">
        Fang compiles from source with cargo and cc. The binary embeds its
        runtime and standard library: no runtime dependencies beyond a C
        compiler.
      </p>
      <div className="steps">
        <div className="step">
          <span className="step-no">01 /</span>
          <div>
            <h4>build the compiler</h4>
            <p>Needs Rust and a C compiler.</p>
            <CodeBlock
              name="shell"
              code={`git clone https://github.com/loak7993-code/fang
cd fang
cargo build --release
cargo install --path crates/fang-cli`}
            />
          </div>
        </div>
        <div className="step">
          <span className="step-no">02 /</span>
          <div>
            <h4>run your first program</h4>
            <CodeBlock name="hello.fg" code={`print("hello, fang")`} output={'hello, fang'} />
          </div>
        </div>
        <div className="step">
          <span className="step-no">03 /</span>
          <div>
            <h4>learn the whole language</h4>
            <p>
              The interactive course runs in your terminal. 18 lessons, a
              real program per lesson.
            </p>
            <CodeBlock name="shell" code={`fang learn`} />
          </div>
        </div>
        <div className="step">
          <span className="step-no">04 /</span>
          <div>
            <h4>everything else</h4>
            <CodeBlock
              name="commands"
              code={`fang run app.fg          # compile and execute
fang build app.fg --release # standalone native binary
fang repl                 # interactive session
fang fmt src/*.fg         # canonical formatting
fang test                 # run every *_test.fg
fang doc                  # 30 topics of reference`}
            />
          </div>
        </div>
      </div>
    </div>
  )
}

/* ---------------- root ---------------- */

export default function App() {
  const [view, setView] = useState<View>(() => {
    const h = window.location.hash.replace('#/', '')
    if (h === 'learn' || h === 'bench' || h === 'cheat' || h === 'install') return h
    return 'home'
  })
  const go = (v: View) => {
    window.location.hash = v === 'home' ? '#/' : '#/' + v
    setView(v)
    window.scrollTo(0, 0)
  }
  window.onhashchange = () => {
    const h = window.location.hash.replace('#/', '')
    if (h === 'learn' || h === 'bench' || h === 'cheat' || h === 'install') setView(h)
    else setView('home')
  }
  return (
    <>
      <div className="glow" aria-hidden />
      <div className="shell">
        <TopBar view={view} go={go} />
        {view === 'home' && <Home go={go} />}
        {view === 'learn' && <Learn />}
        {view === 'bench' && <Bench />}
        {view === 'cheat' && <Cheat />}
        {view === 'install' && <Install />}
        <footer className="footer">
          <div className="footer-inner">
            <div>
              <div className="footer-mark">
                fang<span className="thin">.fg</span>
              </div>
              <p style={{ marginTop: 14 }}>
                Python's flexibility, C's speed. MIT license. Every claim
                on this site is a measured number from the repository's
                test suite.
              </p>
            </div>
            <div className="footer-links">
              <a href={GITHUB} target="_blank" rel="noreferrer">
                github.com/loak7993-code/fang
              </a>
              <a href={GITHUB + '/blob/main/LEARN.md'} target="_blank" rel="noreferrer">
                LEARN.md
              </a>
              <a href={GITHUB + '/blob/main/SPEC.md'} target="_blank" rel="noreferrer">
                SPEC.md
              </a>
              <a href={GITHUB + '/blob/main/STDLIB.md'} target="_blank" rel="noreferrer">
                STDLIB.md
              </a>
              <a href={GITHUB + '/blob/main/CHANGELOG.md'} target="_blank" rel="noreferrer">
                CHANGELOG.md
              </a>
            </div>
          </div>
        </footer>
      </div>
    </>
  )
}
