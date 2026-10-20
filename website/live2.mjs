import velox from 'velox-automation'
const page = await velox.open('https://loak7993-code.github.io/fang/#/', { js: true, engine: 'cdp', viewport: [1440, 900] })
const stats = await page.eval(`document.querySelectorAll('.stat').length`)
const bench = await page.eval(`document.querySelectorAll('.bench-card').length`)
console.log('home stat tiles:', stats)
await page.goto('https://loak7993-code.github.io/fang/#/bench', { waitMs: 600 })
const cards = await page.eval(`document.querySelectorAll('.bench-card').length`)
console.log('bench cards:', cards)
await page.close()
