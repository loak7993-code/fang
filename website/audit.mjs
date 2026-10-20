import velox from 'velox-automation'

const base = 'http://localhost:8930/fang/#/'
const views = ['', 'learn', 'bench', 'cheat', 'install']
const sizes = [
  [390, 844, 'mobile'],
  [768, 1024, 'tablet'],
  [1440, 900, 'desktop'],
]

let failures = 0
for (const [w, h, tag] of sizes) {
  const page = await velox.open(base, { js: true, engine: 'cdp', viewport: [w, h] })
  for (const view of views) {
    await page.goto(base + view, { waitMs: 700 })
    const r = await page.eval(`(() => {
      const out = { problems: [] };
      const de = document.documentElement;
      out.viewport = window.innerWidth + 'x' + window.innerHeight;
      if (de.scrollWidth > window.innerWidth + 1) {
        out.problems.push('horizontal overflow: ' + de.scrollWidth + ' > ' + window.innerWidth);
      }
      const bs = getComputedStyle(document.body);
      out.bg = bs.backgroundColor;
      out.font = bs.fontFamily.slice(0, 24);
      if (!document.querySelector('.nav') || !document.querySelector('.brand')) out.problems.push('missing nav');
      if (!document.querySelector('h1') && !document.querySelector('h2')) out.problems.push('no heading');
      // text clipping check: any element overflowing its own box badly
      const clipped = [];
      for (const el of document.querySelectorAll('h1,h2,h3,p,button,a,pre')) {
        if (el.scrollWidth > el.clientWidth + 4 && el.clientWidth > 0) {
          const st = getComputedStyle(el);
          if (st.overflowX !== 'auto' && st.overflowX !== 'scroll' && st.textOverflow !== 'ellipsis') {
            clipped.push(el.tagName + '.' + (el.className || '').toString().slice(0, 20));
          }
        }
      }
      if (clipped.length) out.problems.push('clipped text: ' + clipped.slice(0, 3).join(', '));
      return out;
    })()`)
    const errs = (page.raw?.console ? await page.raw.console() : []).filter(m => m.type === 'error')
    const status = r.problems.length === 0 && errs.length === 0 ? 'PASS' : 'FAIL'
    if (status === 'FAIL') failures++
    console.log(`${status} ${tag}-${view || 'home'} (${r.viewport}) bg=${r.bg} font=${r.font}`)
    for (const p of r.problems) console.log('   ! ' + p)
    for (const e of errs)
      console.log('   ! console: ' + (typeof e === 'string' ? e : JSON.stringify(e).slice(0, 100)))
  }
  await page.close()
}
console.log(failures === 0 ? 'ALL CLEAN' : failures + ' combos failed')
