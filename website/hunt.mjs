import velox from 'velox-automation'
const page = await velox.open('http://localhost:8930/fang/#/bench', { js: true, engine: 'cdp', viewport: [390, 844] })
const r = await page.eval(`(() => {
  const wide = [];
  for (const el of document.querySelectorAll('*')) {
    const r = el.getBoundingClientRect();
    if (r.width > 391) {
      wide.push(el.tagName + '.' + (el.className || '').toString().slice(0, 30) + ' w=' + Math.round(r.width));
    }
  }
  return wide.slice(0, 8);
})()`)
for (const w of r) console.log(w)
await page.close()
