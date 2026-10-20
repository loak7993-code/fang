import velox from 'velox-automation'
for (const view of ['bench', 'cheat', 'install']) {
  const page = await velox.open('http://localhost:8930/fang/#/' + view, { js: true, engine: 'cdp', viewport: [390, 844] })
  const r = await page.eval(`(() => {
    const de = document.documentElement;
    const out = { sw: de.scrollWidth, over: [] };
    for (const el of document.querySelectorAll('*')) {
      const rect = el.getBoundingClientRect();
      if (rect.right > 391 && rect.width > 0) {
        const st = getComputedStyle(el);
        if (st.display !== 'none' && st.overflowX !== 'auto') {
          out.over.push(el.tagName + '.' + (el.className || '').toString().slice(0, 24) + ' right=' + Math.round(rect.right));
        }
      }
    }
    return out;
  })()`)
  console.log(view, 'scrollWidth=' + r.sw, 'offenders:', r.over.slice(0, 4))
  await page.close()
}
