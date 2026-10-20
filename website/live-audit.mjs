import velox from 'velox-automation'
const base = 'https://loak7993-code.github.io/fang/#/'
const page = await velox.open(base, { js: true, engine: 'cdp', viewport: [1440, 900] })
const r = await page.eval(`(() => {
  const out = { checks: [] };
  out.checks.push(['title', document.title]);
  out.checks.push(['h1', document.querySelector('h1')?.textContent?.trim().slice(0, 40)]);
  out.checks.push(['lessons', document.querySelectorAll('.lesson-item').length]);
  return out;
})()`)
// navigate to learn view and count lessons there
await page.goto(base + 'learn', { waitMs: 800 })
const n = await page.eval(`document.querySelectorAll('.lesson-item').length`)
const stats = await page.eval(`document.querySelectorAll('.stat').length`)
console.log('live title:', JSON.stringify(r.checks[0][1]))
console.log('lesson items on live learn view:', n)
console.log('stat tiles on live home:', stats)
await page.close()
