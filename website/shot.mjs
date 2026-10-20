import velox from 'velox-automation'
import { writeFileSync, mkdirSync } from 'fs'

mkdirSync('/tmp/leo-test/shots', { recursive: true })

const base = 'http://localhost:8930/fang/#/'
const views = ['', 'learn', 'bench', 'cheat', 'install']
const sizes = [
  [390, 844, 'mobile'],
  [768, 1024, 'tablet'],
  [1440, 900, 'desktop'],
]

let totalErrors = []
for (const [w, h, tag] of sizes) {
  const page = await velox.open(base, { js: true, engine: 'cdp', viewport: [w, h] })
  for (const view of views) {
    await page.goto(base + view, { waitMs: 700 })
    await page.goto(base + view, { waitMs: 700 })
    const png = await page.screenshot({ full: false })
    writeFileSync(`/tmp/leo-test/shots/${tag}-${view || 'home'}.png`, png)
  }
  try {
    const errs = await page.consoleErrors()
    if (errs.length) totalErrors = totalErrors.concat(errs)
  } catch {}
  await page.close()
}
console.log('console errors total:', totalErrors.length)
for (const e of totalErrors.slice(0, 6))
  console.log(' -', typeof e === 'string' ? e : e.text || JSON.stringify(e).slice(0, 120))
console.log('done')
