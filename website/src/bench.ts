export interface BenchRow {
  name: string
  fang: number
  python: number | null
  c: number | null
  rust: number | null
  cpp: number | null
  verdict: string
}

// Real medians from bench/run.sh, 2026-10-20, 8-core x86_64.
// Times in seconds; lower is better.
export const BENCH: BenchRow[] = [
  { name: 'fib', fang: 0.009, python: 0.656, c: 0.010, rust: 0.012, cpp: 0.009, verdict: 'C parity' },
  { name: 'loopsum', fang: 0.001, python: 8.511, c: 0.001, rust: 0.002, cpp: 0.002, verdict: 'C parity' },
  { name: 'strbuild', fang: 0.002, python: 0.245, c: null, rust: 0.002, cpp: 0.198, verdict: 'Rust parity, 99x C++' },
  { name: 'dictops', fang: 0.017, python: 0.358, c: null, rust: 0.053, cpp: 0.018, verdict: '3x faster than Rust' },
  { name: 'sort', fang: 0.043, python: 0.971, c: null, rust: 0.042, cpp: 0.084, verdict: '2x std::sort' },
]

export const BUILD_STATS = [
  { num: '91', unit: 'ms', label: 'release build of a server+json program' },
  { num: '61', unit: 'ms', label: 'cold start: compile and run' },
  { num: '33', unit: 'M/s', label: 'CHIP-8 instructions per second' },
  { num: '4.6', unit: 'k/s', label: 'HTTP requests per second, single thread' },
]
