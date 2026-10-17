// alea browser engine — shared by the playground and by works exported with
// `alea --web`. The WebAssembly runs in a Web Worker, so a heavy edition never
// freezes the page, and a new run can cancel the previous one by restarting it.

function aleaWorkerMain() {
  let x = null, wasmBytes = null;
  const files = new Map(), inputs = new Map(), enc = new TextEncoder(), dec = new TextDecoder();

  async function boot() {
    const { instance } = await WebAssembly.instantiate(wasmBytes, {});
    x = instance.exports;
    x.alea_init();
    for (const [p, d] of files) putFile(p, d);
    for (const [n, d] of inputs) putInput(n, d);
  }
  const putInput = (name, data) => {
    const [np, nl] = put(enc.encode(name));
    const [dp, dl] = put(data);
    const rc = x.alea_input_put(np, nl, dp, dl);
    return JSON.parse(dec.decode(out())) || { ok: rc === 0 };
  };
  const put = (bytes) => {
    const p = x.alea_alloc(bytes.length);
    new Uint8Array(x.memory.buffer, p, bytes.length).set(bytes);
    return [p, bytes.length];
  };
  const out = () => new Uint8Array(x.memory.buffer, x.alea_out_ptr(), x.alea_out_len()).slice();
  const putFile = (path, data) => {
    const [pp, pl] = put(enc.encode(path));
    const [dp, dl] = put(data);
    x.alea_fs_put(pp, pl, dp, dl);
  };
  // Runs an export; a crash (a Rust panic) leaves a message and a dead instance.
  const call = (fn) => {
    try {
      const rc = fn();
      return { rc, bytes: out() };
    } catch (e) {
      let msg = String(e);
      try { x.alea_panic_message(); msg = dec.decode(out()) || msg; } catch (_) {}
      x = null;
      return { crash: msg };
    }
  };

  // Messages are handled strictly one after the other: a request that arrives
  // while the engine is still starting waits for it, instead of starting a second one.
  let queue = Promise.resolve();
  self.onmessage = ({ data }) => { queue = queue.then(() => handle(data)); };

  async function handle(m) {
    const reply = (msg, transfer) => self.postMessage({ id: m.id, ...msg }, transfer || []);
    try {
      if (m.type === 'init') {
        wasmBytes = m.wasm;
        for (const f of m.files) files.set(f.path, f.data);
        for (const i of m.inputs || []) inputs.set(i.name, i.data);
        await boot();
        return reply({ ok: true });
      }
      if (!x) await boot();
      if (m.type === 'put') {
        files.set(m.path, m.data);
        putFile(m.path, m.data);
        return reply({ ok: true });
      }
      if (m.type === 'input') {
        // null data: no input of that name any more
        if (m.data) inputs.set(m.name, m.data); else inputs.delete(m.name);
        x.alea_input_clear();
        let res = { ok: true };
        for (const [n, d] of inputs) {
          const r = putInput(n, d);
          if (n === m.name) res = r;
          if (!r.ok) inputs.delete(n);
        }
        return reply({ json: res });
      }
      if (m.type === 'run') {
        const src = enc.encode(m.source);
        files.set('/' + m.path, src);
        putFile('/' + m.path, src);
        const seed = BigInt(m.seed);
        const [pp, pl] = put(enc.encode(m.path));
        const r = call(() => x.alea_run(pp, pl, Number(seed >> 32n), Number(seed & 0xffffffffn)));
        if (r.crash) return reply({ crash: r.crash });
        return reply({ json: JSON.parse(dec.decode(r.bytes)) });
      }
      const fns = {
        pixels: () => x.alea_pixels(m.index, m.w, m.h, m.t),
        sound: () => x.alea_sound(m.index),
        stl: () => x.alea_stl(m.index),
        png: () => x.alea_png(m.index),
        midi: () => x.alea_midi(m.index),
        score: () => x.alea_score(m.index),
      };
      const r = call(fns[m.type]);
      if (r.crash) return reply({ crash: r.crash });
      if (r.rc !== 0) return reply({ json: JSON.parse(dec.decode(r.bytes)) });
      return reply({ bytes: r.bytes.buffer }, [r.bytes.buffer]);
    } catch (e) {
      reply({ crash: String(e && e.message || e) });
    }
  }
}

class AleaEngine {
  constructor(wasmBytes, files) {
    this.wasm = wasmBytes;           // Uint8Array
    this.files = files;              // [{path, data: Uint8Array}]
    this.inputs = [];                // [{name, data: Uint8Array}]: what a pipe brings
    this.start();
  }
  start() {
    const src = '(' + aleaWorkerMain.toString() + ')()';
    this.worker = new Worker(URL.createObjectURL(new Blob([src], { type: 'text/javascript' })));
    this.pending = new Map();
    this.nextId = 1;
    this.worker.onmessage = ({ data }) => {
      const p = this.pending.get(data.id);
      if (!p) return;
      this.pending.delete(data.id);
      p.resolve(data);
    };
    this.worker.onerror = (e) => {
      for (const p of this.pending.values()) p.resolve({ crash: e.message || 'the engine stopped' });
      this.pending.clear();
    };
    this.ready = this.request({
      type: 'init',
      wasm: this.wasm.slice(),
      files: this.files.map((f) => ({ path: f.path, data: f.data.slice() })),
      inputs: this.inputs.map((i) => ({ name: i.name, data: i.data.slice() })),
    });
  }
  /** Cancels whatever is running by starting a fresh worker. */
  restart() {
    this.worker.terminate();
    for (const p of this.pending.values()) p.resolve({ cancelled: true });
    this.start();
  }
  request(msg) {
    const id = this.nextId++;
    return new Promise((resolve) => {
      this.pending.set(id, { resolve });
      this.worker.postMessage({ id, ...msg });
    });
  }
  /** Gives the work an input ("" is the default one), or removes it (data null). */
  async setInput(name, data) {
    const r = await this.request({ type: 'input', name, data: data ? data.slice() : null });
    this.inputs = this.inputs.filter((i) => i.name !== name);
    if (data && r.json && r.json.ok) this.inputs.push({ name, data });
    return r.json || { ok: false, message: r.crash || 'the engine stopped' };
  }
  addFile(path, data) {
    this.files = this.files.filter((f) => f.path !== path).concat([{ path, data }]);
    return this.request({ type: 'put', path, data: data.slice() });
  }
}

// ───────────── helpers shared by the pages ─────────────

/** A score from the engine: { duration, msgs: [{ at, bytes }] }. */
function aleaScore(bytes) {
  const v = new DataView(bytes instanceof ArrayBuffer ? bytes : bytes.buffer);
  const duration = v.getFloat32(0, true), msgs = [];
  for (let o = 4; o + 8 <= v.byteLength; o += 8) {
    const n = v.getUint8(o + 4);
    msgs.push({ at: v.getFloat32(o, true), bytes: [v.getUint8(o + 5), v.getUint8(o + 6), v.getUint8(o + 7)].slice(0, n) });
  }
  return { duration, msgs };
}

/** The notes of a score, paired: [{ ch, note, vel, start, end }]. */
function aleaNotes(score) {
  const open = new Map(), out = [];
  for (const m of score.msgs) {
    const [st, n, v] = m.bytes, kind = st & 0xf0, ch = st & 0x0f, key = ch * 128 + n;
    if (kind === 0x90 && v > 0) { open.set(key, { ch, note: n, vel: v, start: m.at }); }
    else if (kind === 0x80 || kind === 0x90) { const o = open.get(key); if (o) { o.end = m.at; out.push(o); open.delete(key); } }
  }
  for (const o of open.values()) { o.end = score.duration; out.push(o); }
  return out;
}

function aleaRandomSeed() {
  const a = new Uint32Array(2);
  crypto.getRandomValues(a);
  return '0x' + ((BigInt(a[0]) << 32n) | BigInt(a[1])).toString(16).padStart(16, '0');
}

function aleaBase64ToBytes(b64) {
  const bin = atob(b64.replace(/\s+/g, ''));
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** The channels of a sound from the engine: a count, then each channel in turn. */
function aleaChannels(bytes) {
  const f = new Float32Array(bytes), nc = Math.max(1, Math.round(f[0])), len = (f.length - 1) / nc;
  return Array.from({ length: nc }, (_, c) => f.subarray(1 + c * len, 1 + (c + 1) * len));
}

/** 16-bit WAV (mono or stereo) from channels of float samples in [-1, 1]. */
function aleaWav(channels, rate = 44100) {
  if (!Array.isArray(channels)) channels = [channels];
  const nc = channels.length, n = channels[0].length, bytes = n * nc * 2;
  const buf = new ArrayBuffer(44 + bytes), v = new DataView(buf);
  const str = (o, s) => [...s].forEach((c, i) => v.setUint8(o + i, c.charCodeAt(0)));
  str(0, 'RIFF'); v.setUint32(4, 36 + bytes, true); str(8, 'WAVEfmt ');
  v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, nc, true);
  v.setUint32(24, rate, true); v.setUint32(28, rate * 2 * nc, true); v.setUint16(32, 2 * nc, true); v.setUint16(34, 16, true);
  str(36, 'data'); v.setUint32(40, bytes, true);
  for (let i = 0; i < n; i++) for (let c = 0; c < nc; c++) {
    v.setInt16(44 + (i * nc + c) * 2, Math.trunc(Math.max(-1, Math.min(1, channels[c][i])) * 32767), true);
  }
  return new Uint8Array(buf);
}

/** An AudioBuffer holding the channels. */
function aleaAudioBuffer(ctx, channels) {
  const buf = ctx.createBuffer(channels.length, channels[0].length, 44100);
  channels.forEach((c, k) => buf.copyToChannel(c, k));
  return buf;
}

/** A ZIP archive (stored, uncompressed) of [{name, data: Uint8Array}]. */
function aleaZip(entries) {
  const crcTable = aleaZip.t || (aleaZip.t = Array.from({ length: 256 }, (_, n) => {
    let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xEDB88320 ^ (c >>> 1) : c >>> 1; return c >>> 0;
  }));
  const crc32 = (d) => { let c = 0xFFFFFFFF; for (let i = 0; i < d.length; i++) c = crcTable[(c ^ d[i]) & 0xFF] ^ (c >>> 8); return (c ^ 0xFFFFFFFF) >>> 0; };
  const enc = new TextEncoder(), parts = [], central = [];
  let offset = 0;
  for (const e of entries) {
    const name = enc.encode(e.name), crc = crc32(e.data), size = e.data.length;
    const local = new DataView(new ArrayBuffer(30));
    local.setUint32(0, 0x04034b50, true); local.setUint16(4, 20, true); local.setUint16(8, 0, true);
    local.setUint32(14, crc, true); local.setUint32(18, size, true); local.setUint32(22, size, true);
    local.setUint16(26, name.length, true);
    parts.push(new Uint8Array(local.buffer), name, e.data);
    const c = new DataView(new ArrayBuffer(46));
    c.setUint32(0, 0x02014b50, true); c.setUint16(4, 20, true); c.setUint16(6, 20, true);
    c.setUint32(16, crc, true); c.setUint32(20, size, true); c.setUint32(24, size, true);
    c.setUint16(28, name.length, true); c.setUint32(42, offset, true);
    central.push(new Uint8Array(c.buffer), name);
    offset += 30 + name.length + size;
  }
  const cdSize = central.reduce((s, p) => s + p.length, 0);
  const end = new DataView(new ArrayBuffer(22));
  end.setUint32(0, 0x06054b50, true); end.setUint16(8, entries.length, true); end.setUint16(10, entries.length, true);
  end.setUint32(12, cdSize, true); end.setUint32(16, offset, true);
  const all = [...parts, ...central, new Uint8Array(end.buffer)];
  const out = new Uint8Array(all.reduce((s, p) => s + p.length, 0));
  let o = 0; for (const p of all) { out.set(p, o); o += p.length; }
  return out;
}

/**
 * Saves a file: through the artifact viewer's `downloads` capability when the
 * page runs inside claude.ai, with a plain download link when it is a local file.
 * Returns false when saving is unavailable here.
 */
async function aleaSave(filename, bytes) {
  if (window.claude && typeof window.claude.use === 'function') {
    const dl = await window.claude.use('downloads');
    if (!dl) return false;
    try { await dl.save({ filename, data: new Blob([bytes]) }); } catch (_) { /* declined */ }
    return true;
  }
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([bytes]));
  a.download = filename;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 4000);
  return true;
}
