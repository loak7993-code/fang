import velox from 'velox-automation'
const page = await velox.open('https://loak7993-code.github.io/fang/#/', { js: true, engine: 'cdp', viewport: [1440, 900] })
const r = await page.eval(`(() => ({
  wordmark: !!document.querySelector('.wordmark'),
  term: !!document.querySelector('.term'),
  cursor: !!document.querySelector('.cursor'),
  rows: document.querySelectorAll('.row').length,
  stripCells: document.querySelectorAll('.strip-cell').length,
  sw: document.documentElement.scrollWidth,
  bg: getComputedStyle(document.body).backgroundColor,
}))()`)
console.log(JSON.stringify(r))
await page.goto('https://loak7993-code.github.io/fang/#/learn', { waitMs: 700 })
const l = await page.eval(`document.querySelectorAll('.lesson-index button').length`)
console.log('live lessons:', l)
await page.close()
