import { useState } from 'react'
import {
  TerminalWindow,
  Globe,
  Monitor,
  Cpu,
  Plugs,
  Planet,
  Lightbulb,
  ArrowLeft,
  ArrowRight,
} from '@phosphor-icons/react'
import { COURSE } from './course'
import { BENCH, BUILD_STATS } from './bench'

type View = 'home' | 'learn' | 'bench' | 'cheat' | 'install'

const GITHUB = 'https://github.com/loak7993-code/fang'

function CodeBlock({
  name,
  code,
  output,
}: {
  name: string
  code: string
  output?: string
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
  const render = code.split('___')
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
        {render.map((part, i) => (
          <span key={i}>
            {part}
            {i < render.length - 1 && <span className="blank">___</span>}
          </span>
        ))}
      </pre>
      {output !== undefined && (
        <>
          <div className="output-label">output</div>
          <div className="output">{output.replace(/\n$/, '')}</div>
        </>
      )}
    </div>
  )
}

function Logo({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden>
      <rect width="32" height="32" rx="8" fill="#141416" />
      <path d="M7 9 L14 23 L16 18 L12 9 Z" fill="#34d399" />
      <path d="M25 9 L18 23 L16 18 L20 9 Z" fill="#0d9488" />
    </svg>
  )
}

function Nav({ view, go }: { view: View; go: (v: View) => void }) {
  const items: [View, string][] = [
    ['home', 'Home'],
    ['learn', 'Learn'],
    ['bench', 'Benchmarks'],
    ['cheat', 'Cheat sheet'],
    ['install', 'Install'],
  ]
  return (
    <nav className="nav">
      <div className="nav-inner">
        <a
          className="brand"
          href="#/"
          onClick={(e) => {
            e.preventDefault()
            go('home')
          }}
        >
          <Logo />
          fang
        </a>
        <div className="nav-links">
          {items.map(([v, label]) => (
            <a
              key={v}
              href={'#/' + (v === 'home' ? '' : v)}
              className={'nav-link' + (view === v ? ' active' : '')}
              onClick={(e) => {
                e.preventDefault()
                go(v)
              }}
            >
              {label}
            </a>
          ))}
          <a className="nav-cta" href={GITHUB} target="_blank" rel="noreferrer">
            GitHub
          </a>
        </div>
      </div>
    </nav>
  )
}

function Home({ go }: { go: (v: View) => void }) {
  return (
    <div className="page">
      <section className="hero">
        <div className="pill-row">
          <span className="pill">v1.4.1</span>
          <span className="pill">compiles to C</span>
          <span className="pill">learn in an hour</span>
        </div>
        <h1 style={{ marginTop: 20 }}>
          Python's flexibility.
          <br />
          <span className="em">C's speed.</span>
        </h1>
        <p className="lede">
          Fang is a programming language that compiles through C to native
          code. Scripts with zero ceremony, servers, terminal apps,
          emulators, kernels. One language, native everywhere C ships.
        </p>
        <div className="hero-actions">
          <button className="btn btn-primary" onClick={() => go('learn')}>
            Start the course
          </button>
          <button className="btn btn-ghost" onClick={() => go('install')}>
            Install
          </button>
        </div>
      </section>

      <section className="section">
        <CodeBlock
          name="hello.fg"
          code={'print("hello, fang")'}
          output={'hello, fang'}
        />
        <div style={{ marginTop: 16 }}>
          <CodeBlock
            name="server.fg"
            code={`import server

fn handler(req: Request):
  if req.path == "/":
    return server.html("<h1>hello from fang</h1>")
  return server.not_found()

server.serve(handler, 8080)`}
            output={'fang serving on http://localhost:8080'}
          />
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">Measured, not claimed</h2>
        <p className="section-sub">
          Every number on this page comes from the benchmark suite in the
          repository, run on real hardware.
        </p>
        <div className="stats">
          {BUILD_STATS.map((s) => (
            <div className="stat" key={s.label}>
              <div className="num">
                {s.num} <span className="unit">{s.unit}</span>
              </div>
              <div className="label">{s.label}</div>
            </div>
          ))}
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">What you can build</h2>
        <div className="feature-grid">
          <Feature title="Scripts and CLI tools" text="Plain lines in a file, run top to bottom. No fn main, no return keyword, types inferred." icon="terminal" />
          <Feature title="Backend websites" text="The HTTP server is pure Fang: handler is a plain function value, GET and POST verified with curl." icon="globe" />
          <Feature title="Terminal apps" text="The tui module draws screens, colors, and reads keys with zero dependencies. CHIP-8 runs at 33M instructions per second." icon="monitor" />
          <Feature title="Kernels and bare metal" text="freestanding builds emit C with no runtime and no libc, pass any target triple to the platform compiler." icon="cpu" />
          <Feature title="Python libraries" text="import py bridges CPython when you need numpy or the ecosystem, opt-in at link time." icon="plugs" />
          <Feature title="Every OS" text="C reaches Linux, Windows, macOS, the BSDs, Android and iOS; cross-compile by target triple." icon="planet" />
        </div>
      </section>

      <section className="section">
        <h2 className="section-title">The language in one minute</h2>
        <div style={{ maxWidth: 720 }}>
          <CodeBlock
            name="tour.fg"
            code={`# scripts are just lines
name = "world"
print(f"hello {name}")

# functions: last expression is the return value
fn double(n):
  n * 2

# module-level let is a mutable global, like Python
let hits: list[int] = []

fn track(n):
  hits.push(n)
  len(hits)

track(1)
track(2)
print(track(3))          # 3

# comprehensions, closures, structs, dict
evens = [x * 2 for x in 0..5]
add = fn(a, b): a + b
print(evens, add(3, 4)) # [0, 2, 4, 6, 8] 7

struct Point:
  x: float
  y: float

fn Point.hypot(self):
  (self.x * self.x + self.y * self.y) ** 0.5

p = Point(3.0, 4.0)
print(p.hypot())        # 5`}
            output={`hello world
3
[0, 2, 4, 6, 8] 7
5`}
          />
        </div>
      </section>
    </div>
  )
}

function Feature({
  title,
  text,
  icon,
}: {
  title: string
  text: string
  icon: string
}) {
  const icons: Record<string, React.ReactNode> = {
    terminal: <TerminalWindow weight="bold" />,
    globe: <Globe weight="bold" />,
    monitor: <Monitor weight="bold" />,
    cpu: <Cpu weight="bold" />,
    plugs: <Plugs weight="bold" />,
    planet: <Planet weight="bold" />,
  }
  return (
    <div className="feature">
      <div className="icon" style={{ fontSize: 24 }}>
        {icons[icon]}
      </div>
      <h3>{title}</h3>
      <p>{text}</p>
    </div>
  )
}

function Learn() {
  const [idx, setIdx] = useState(0)
  const lesson = COURSE[idx]
  const titles = lesson.title.split('. ')
  return (
    <div className="page">
      <div className="learn-layout">
        <aside className="lesson-list" aria-label="Lessons">
          {COURSE.map((l, i) => (
            <button
              key={l.n}
              className={'lesson-item' + (i === idx ? ' active' : '')}
              onClick={() => setIdx(i)}
            >
              <span className="n">{l.n}</span>
              <span>{titles.length > 1 ? titles[1] : l.title}</span>
            </button>
          ))}
        </aside>
        <div className="lesson" key={idx}>
          <span className="pill">lesson {lesson.n} of 18</span>
          <h2 style={{ marginTop: 14 }}>{lesson.title}</h2>
          <p className="teach">{lesson.teach}</p>
          <div className="stack">
            <CodeBlock name={'lesson' + lesson.n + '.fg'} code={lesson.program} />
            <CodeBlock name="expected output" code={lesson.expect.trimEnd()} />
          </div>
          <details className="hint-box">
            <summary>
              <Lightbulb weight="bold" /> stuck? show the hint
            </summary>
            <p style={{ marginTop: 10 }}>{lesson.hint}</p>
          </details>
          <div className="recap">{lesson.recap}</div>
          <div className="lesson-nav">
            <button
              className="lesson-btn"
              disabled={idx === 0}
              onClick={() => setIdx(idx - 1)}
            >
              <ArrowLeft weight="bold" /> previous
            </button>
            <button
              className="lesson-btn"
              disabled={idx === COURSE.length - 1}
              onClick={() => setIdx(idx + 1)}
            >
              next <ArrowRight weight="bold" />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

function Bench() {
  return (
    <div className="page">
      <h2 className="section-title">Benchmarks</h2>
      <p className="section-sub">
        Median of 5 runs, seconds, lower is better. Fang builds at -O3
        -march=native, same as the C and Rust baselines. Full harness in{' '}
        <a href={GITHUB + '/tree/main/bench'} target="_blank" rel="noreferrer" style={{ color: 'var(--accent)' }}>
          bench/
        </a>
        .
      </p>
      <div className="bench-grid">
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
            <div className="bench-card" key={b.name}>
              <h3>
                {b.name}
                <span className="verdict">{b.verdict}</span>
              </h3>
              {valid.map(([lang, t, isFang]) => (
                <div className={'bar-row' + (isFang ? ' fang' : '')} key={lang}>
                  <span className="lang">{lang}</span>
                  <div className="bar-track">
                    <div className="bar-fill" style={{ width: `${(t / max) * 100}%` }} />
                  </div>
                  <span className="val">{t.toFixed(3)}s</span>
                </div>
              ))}
            </div>
          )
        })}
      </div>
      <section className="section">
        <h2 className="section-title">Toolchain</h2>
        <p className="section-sub">
          Incremental compilation: modules compile once and are cached as
          object files; only your entry file compiles on every build.
        </p>
        <div className="stats">
          <div className="stat">
            <div className="num">91<span className="unit">ms</span></div>
            <div className="label">fang build --release, server+json program</div>
          </div>
          <div className="stat">
            <div className="num">108<span className="unit">ms</span></div>
            <div className="label">rustc -O on the same size program</div>
          </div>
          <div className="stat">
            <div className="num">61<span className="unit">ms</span></div>
            <div className="label">fang run cold start, compile and execute</div>
          </div>
          <div className="stat">
            <div className="num">50<span className="unit">KB</span></div>
            <div className="label">static binary for a hello world</div>
          </div>
        </div>
      </section>
    </div>
  )
}

const CHEAT: [string, string][] = [
  ['variables', `x = 5              # plain assignment
let hits: list[int] = []   # module-level global
name = "fang"`],
  ['control flow', `if x > 10:
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
  ['functions', `fn double(n):
  n * 2              # last expression returns

fn fib(n: int) -> int:   # recursion declares the type
  if n < 2:
    return n
  fib(n - 1) + fib(n - 2)

fn Point.hypot(self):   # method
  (self.x ** 2 + self.y ** 2) ** 0.5

add = fn(a, b): a + b   # lambda value`],
  ['collections', `xs = [1, 2, 3]
xs.push(4)
evens = [x * 2 for x in xs if x % 2 == 0]

ages = {"panda": 30, "fang": 1}
ages["nova"] = 5
for k, v in ages.items():
  print(k, v)`],
  ['structs and errors', `struct Point:
  x: float
  y: float

try:
  risky()
except err:
  print("caught", err)`],
  ['bitwise and hex', `mask = 0xF0 | 0x0F
low = byte & 15
wide = 1 << 8
bits = 0b1010`],
  ['modules', `import server
import str

fn handler(req: Request) -> Response:
  if req.path == "/":
    return server.html("<h1>hi</h1>")
  return server.not_found()`],
  ['tui and keys', `import tui

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
    <div className="page">
      <h2 className="section-title">Cheat sheet</h2>
      <p className="section-sub">
        The whole language fits on one screen. Same content ships in the
        compiler as <code style={{ fontFamily: 'var(--font-mono)' }}>fang cheat</code>.
      </p>
      <div className="cheat-grid">
        {CHEAT.map(([name, code]) => (
          <div className="cheat-card" key={name}>
            <h3>{name}</h3>
            <pre>{code}</pre>
          </div>
        ))}
      </div>
    </div>
  )
}

function Install() {
  return (
    <div className="page">
      <h2 className="section-title">Install</h2>
      <p className="section-sub">
        Fang compiles from source with cargo and cc. The binary embeds its
        runtime and standard library: no runtime dependencies beyond a C
        compiler.
      </p>
      <div className="install-steps">
        <div className="step">
          <span className="step-n">1</span>
          <div style={{ flex: 1 }}>
            <h4>Build the compiler</h4>
            <p>Needs Rust and a C compiler.</p>
            <CodeBlock
              name="shell"
              code={'git clone https://github.com/loak7993-code/fang\ncd fang\ncargo build --release\ncargo install --path crates/fang-cli'}
            />
          </div>
        </div>
        <div className="step">
          <span className="step-n">2</span>
          <div style={{ flex: 1 }}>
            <h4>Run your first program</h4>
            <CodeBlock
              name="hello.fg"
              code={'print("hello, fang")'}
              output={'hello, fang'}
            />
          </div>
        </div>
        <div className="step">
          <span className="step-n">3</span>
          <div style={{ flex: 1 }}>
            <h4>Learn the whole language</h4>
            <p>
              The interactive course runs in your terminal, 18 lessons, a
              real program per lesson.
            </p>
            <CodeBlock name="shell" code={'fang learn'} />
          </div>
        </div>
        <div className="step">
          <span className="step-n">4</span>
          <div style={{ flex: 1 }}>
            <h4>Everything else</h4>
            <CodeBlock
              name="commands"
              code={'fang run app.fg          # compile and execute\nfang build app.fg --release # standalone native binary\nfang repl                 # interactive session\nfang fmt src/*.fg         # canonical formatting\nfang test                 # run every *_test.fg\nfang doc                  # 30 topics of reference'}
            />
          </div>
        </div>
      </div>
    </div>
  )
}

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
        <Nav view={view} go={go} />
        {view === 'home' && <Home go={go} />}
        {view === 'learn' && <Learn />}
        {view === 'bench' && <Bench />}
        {view === 'cheat' && <Cheat />}
        {view === 'install' && <Install />}
        <footer className="footer">
          <div className="footer-inner">
            <div>
              <p>
                Fang 1.4.1, MIT license. Python's flexibility, C's speed.
              </p>
              <p style={{ marginTop: 6 }}>
                Every claim on this site is a measured number from the repository's test suite.
              </p>
            </div>
            <div className="links">
              <a href={GITHUB} target="_blank" rel="noreferrer">GitHub</a>
              <a href={GITHUB + '/blob/main/LEARN.md'} target="_blank" rel="noreferrer">LEARN.md</a>
              <a href={GITHUB + '/blob/main/SPEC.md'} target="_blank" rel="noreferrer">SPEC.md</a>
              <a href={GITHUB + '/blob/main/STDLIB.md'} target="_blank" rel="noreferrer">STDLIB.md</a>
            </div>
          </div>
        </footer>
      </div>
    </>
  )
}
