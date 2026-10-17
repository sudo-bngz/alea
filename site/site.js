// alea.bngz.io: highlights the code, and draws a new edition of Isohypses with
// the real engine (alea.wasm), in the visitor's browser.
(() => {
  'use strict';
  const $ = (id) => document.getElementById(id);

  // ── a small highlighter for alea ──
  const KEYWORDS = /^(work|let|field|output|render|use|choose|for|in|if|else|constraint|true|false)$/;
  function highlight(src) {
    const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    const re = /(\/\/[^\n]*)|("(?:[^"\\]|\\.)*")|(~~?)|(\b\d+(?:\.\d+)?\b|\b0x[0-9a-f]+\b)|([A-Za-z_][A-Za-z0-9_]*)(\s*\()?|([\s\S])/gi;
    let out = '', m;
    while ((m = re.exec(src))) {
      if (m[1]) out += `<span class="c">${esc(m[1])}</span>`;
      else if (m[2]) out += `<span class="s">${esc(m[2])}</span>`;
      else if (m[3]) out += `<span class="t">${m[3]}</span>`;
      else if (m[4]) out += `<span class="n">${m[4]}</span>`;
      else if (m[5]) {
        if (KEYWORDS.test(m[5])) out += `<span class="k">${m[5]}</span>`;
        else if (m[6]) out += `<span class="f">${m[5]}</span>${esc(m[6])}`;
        else out += m[5];
        if (KEYWORDS.test(m[5]) && m[6]) out += esc(m[6]);
      } else out += esc(m[7]);
    }
    return out;
  }
  for (const el of document.querySelectorAll('pre code.alea')) {
    if (el.id !== 'code') el.innerHTML = highlight(el.textContent);
  }

  // ── the live edition ──
  const canvas = $('live'), still = $('still'), seedEl = $('seed'), status = $('status'), again = $('again');
  const say = (text) => { status.textContent = text; };
  let engine = null, source = null, drawing = false;

  async function draw() {
    if (!engine || drawing) return;
    drawing = true;
    again.disabled = true;
    const seed = aleaRandomSeed();
    say('drawing a new edition…');
    const t0 = performance.now();
    try {
      const res = await engine.request({ type: 'run', path: '/isohypses.alea', source, seed });
      if (res.crash || !res.json || !res.json.ok) throw new Error(res.crash || (res.json && res.json.message) || 'no edition');
      const i = res.json.renders.findIndex((r) => r.kind === 'image');
      const r = res.json.renders[i];
      // as many pixels as the page shows, up to the work's own size
      const shown = canvas.parentElement.clientWidth * Math.min(window.devicePixelRatio || 1, 2);
      const w = Math.min(r.width, Math.max(320, Math.round(shown)));
      const h = Math.round(w * r.height / r.width);
      const px = await engine.request({ type: 'pixels', index: i, w, h, t: -1 });
      if (!px.bytes) throw new Error('no pixels');
      canvas.width = w; canvas.height = h;
      canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(px.bytes), w, h), 0, 0);
      canvas.hidden = false;
      still.hidden = true;
      seedEl.textContent = res.json.seed;
      say(`drawn in your browser in ${((performance.now() - t0) / 1000).toFixed(1)} s`);
    } catch (e) {
      say('your browser could not draw an edition; this one is from the exhibition');
    }
    drawing = false;
    again.disabled = false;
  }

  async function start() {
    try {
      const [src, wasm] = await Promise.all([
        fetch('works/isohypses.alea').then((r) => r.text()),
        fetch('alea.wasm').then((r) => r.arrayBuffer()),
      ]);
      source = src;
      $('code').innerHTML = highlight(src);
      engine = new AleaEngine(new Uint8Array(wasm), [{ path: '/isohypses.alea', data: new TextEncoder().encode(src) }]);
      const ready = await engine.ready;
      if (ready.crash) throw new Error(ready.crash);
      again.addEventListener('click', draw);
      await draw();
    } catch (e) {
      say('an edition from the exhibition');
      if (source === null) $('code').textContent = 'The program could not be loaded.';
    }
  }
  if (typeof AleaEngine === 'function' && typeof WebAssembly === 'object') start();
})();
