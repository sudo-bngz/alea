//! Browser entry points (WebAssembly).
//!
//! A plain C interface, so the page needs no generated glue: JavaScript copies
//! bytes in with `alea_alloc` (ownership passes to Rust), calls a function, and
//! reads the result from `alea_out_ptr` / `alea_out_len`. Every call that can
//! fail returns 0 on success, or 1 with a JSON error in the output buffer.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Error;
use crate::eval::{self, Edition};
use crate::library::{self, Module};
use crate::media;

struct State {
    module: Arc<Module>,
    edition: Edition,
    /// One set of colour bounds per animation, so frames don't flicker.
    anim_bounds: HashMap<usize, (f64, f64)>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_out(v: Vec<u8>) {
    OUT.with(|o| *o.borrow_mut() = v);
}

/// Takes ownership of a buffer filled by JavaScript after `alea_alloc`.
unsafe fn take(ptr: *mut u8, len: usize) -> Vec<u8> {
    if len == 0 {
        return Vec::new();
    }
    unsafe { Vec::from_raw_parts(ptr, len, len) }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Writes an error as JSON: message, the formatted diagnostic, and where it points.
fn fail(e: Error, main_src: &str, main_file: &str) -> u32 {
    let text = e.display(main_src, main_file);
    let file = e.origin.as_ref().map(|(f, _)| f.as_str()).unwrap_or(main_file);
    let pos = match e.span {
        Some(s) => format!(r#","line":{},"col":{},"len":{}"#, s.line, s.col, s.len),
        None => String::new(),
    };
    set_out(
        format!(r#"{{"ok":false,"message":{},"text":{},"file":{}{pos}}}"#, json_str(&e.msg), json_str(&text), json_str(file))
            .into_bytes(),
    );
    1
}

fn with_state<F: FnOnce(&mut State) -> Result<Vec<u8>, Error>>(f: F) -> u32 {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(st) = s.as_mut() else {
            return fail(Error::bare("nothing has been run yet"), "", "");
        };
        match f(st) {
            Ok(v) => {
                set_out(v);
                0
            }
            Err(e) => {
                let (src, file) = (st.module.src.clone(), st.module.file.clone());
                fail(e, &src, &file)
            }
        }
    })
}

fn render_at(st: &State, index: usize) -> Result<&eval::Render, Error> {
    st.edition.renders.get(index).ok_or_else(|| Error::bare(format!("there is no render number {index}")))
}

fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgb.len() / 3 * 4);
    for px in rgb.chunks_exact(3) {
        out.extend_from_slice(&[px[0], px[1], px[2], 255]);
    }
    out
}

// ───────────────────────────── exports ─────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn alea_init() {
    crate::vfs::use_memory();
    std::panic::set_hook(Box::new(|info| {
        PANIC.with(|p| *p.borrow_mut() = info.to_string());
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn alea_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[unsafe(no_mangle)]
pub extern "C" fn alea_out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn alea_out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

/// After a crash: the panic message, in the output buffer.
#[unsafe(no_mangle)]
pub extern "C" fn alea_panic_message() {
    let msg = PANIC.with(|p| p.borrow().clone());
    set_out(msg.into_bytes());
}

/// Adds a file (a work, a picture, a font) to the in-memory file system.
///
/// # Safety
/// Both buffers must come from `alea_alloc` and be fully written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alea_fs_put(path: *mut u8, path_len: usize, data: *mut u8, data_len: usize) {
    let path = String::from_utf8_lossy(&unsafe { take(path, path_len) }).into_owned();
    crate::vfs::put(&path, unsafe { take(data, data_len) });
}

#[unsafe(no_mangle)]
pub extern "C" fn alea_fs_clear() {
    crate::vfs::clear();
}

/// Gives the work an input (`name` "" for the default one, what a pipe would
/// bring on the command line). Returns 0, or 1 with an error message as output.
///
/// # Safety
/// Both buffers must come from `alea_alloc` and be fully written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alea_input_put(name: *mut u8, name_len: usize, data: *mut u8, data_len: usize) -> u32 {
    let name = String::from_utf8_lossy(&unsafe { take(name, name_len) }).into_owned();
    match crate::input::put(&name, unsafe { take(data, data_len) }) {
        Ok(i) => {
            let what = match &i.kind {
                crate::input::Kind::Image(img) => format!("an image, {}×{}", img.w, img.h),
                crate::input::Kind::Sound(c) => format!("a sound, {:.1} s", c.duration()),
                crate::input::Kind::Text(t) => format!("a text, {} words", t.split_whitespace().count()),
            };
            set_out(format!(r#"{{"ok":true,"what":{}}}"#, json_str(&what)).into_bytes());
            0
        }
        Err(e) => {
            set_out(format!(r#"{{"ok":false,"message":{}}}"#, json_str(&e)).into_bytes());
            1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alea_input_clear() {
    crate::input::clear();
}

/// Loads `path`, draws the edition `seed`, and checks every render.
/// On success the output is a JSON description of the edition.
///
/// # Safety
/// `path` must come from `alea_alloc` and be fully written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alea_run(path: *mut u8, path_len: usize, seed_hi: u32, seed_lo: u32) -> u32 {
    let path = String::from_utf8_lossy(&unsafe { take(path, path_len) }).into_owned();
    let seed = ((seed_hi as u64) << 32) | seed_lo as u64;
    STATE.with(|s| *s.borrow_mut() = None);

    let module = match library::load(&path) {
        Ok(m) => m,
        Err(e) => return fail(e, "", &path),
    };
    let edition = match eval::resolve(&module, seed).and_then(|ed| media::validate(&ed).map(|_| ed)) {
        Ok(ed) => ed,
        Err(e) => return fail(e, &module.src, &module.file),
    };

    let mut renders = Vec::new();
    for r in &edition.renders {
        let desc = match r.kind.as_str() {
            "image" => {
                let s = media::image_spec(r).expect("validated");
                format!(r#"{{"kind":"image","width":{},"height":{}}}"#, s.w, s.h)
            }
            "sound" => {
                let d = media::sound_duration(r).expect("validated");
                format!(r#"{{"kind":"sound","duration":{}}}"#, d)
            }
            "video" => {
                let s = media::video_spec(r).expect("validated");
                format!(
                    r#"{{"kind":"video","width":{},"height":{},"fps":{},"duration":{},"audio":{},"gif":{}}}"#,
                    s.w, s.h, s.fps, s.duration, s.audio, s.gif
                )
            }
            "midi" => {
                let s = media::midi_spec(r).expect("validated");
                format!(r#"{{"kind":"midi","duration":{}}}"#, s.duration)
            }
            _ => {
                let s = media::sculpture_spec(r).expect("validated");
                format!(r#"{{"kind":"sculpture","resolution":{}}}"#, s.n)
            }
        };
        renders.push(desc);
    }
    set_out(
        format!(
            r#"{{"ok":true,"name":{},"seed":"0x{:016x}","attempts":{},"renders":[{}]}}"#,
            json_str(&edition.name),
            edition.seed,
            edition.attempts,
            renders.join(",")
        )
        .into_bytes(),
    );
    STATE.with(|s| *s.borrow_mut() = Some(State { module, edition, anim_bounds: HashMap::new() }));
    0
}

/// RGBA pixels of render `index` at w×h. A video is drawn at time `t`; an
/// image at its own `at`; a sculpture gives its shaded relief at its own
/// resolution (w and h are ignored).
#[unsafe(no_mangle)]
pub extern "C" fn alea_pixels(index: usize, w: usize, h: usize, t: f64) -> u32 {
    with_state(|st| {
        let seed = st.edition.seed;
        let r = render_at(st, index)?;
        let (w, h) = (w.clamp(1, 4096), h.clamp(1, 4096));
        Ok(match r.kind.as_str() {
            "image" => {
                let s = media::image_spec(r)?;
                rgb_to_rgba(&r.visual("the field to paint")?.rgb(w, h, s.at, seed, &s.stops, s.range))
            }
            "video" => {
                let s = media::video_spec(r)?;
                if let media::Visual::Paint(p) = r.visual("the field to animate")? {
                    return Ok(rgb_to_rgba(&crate::paint::frame_rgb(p, w, h, t)));
                }
                let f = r.required_field(0, "the field to animate")?.clone();
                let b = match st.anim_bounds.get(&index) {
                    Some(b) => *b,
                    None => {
                        let b = s.range.unwrap_or_else(|| media::animation_bounds(&f, s.w, s.h, seed));
                        st.anim_bounds.insert(index, b);
                        b
                    }
                };
                rgb_to_rgba(&media::frame_rgb(&f, w, h, t, seed, &s.stops, Some(b)))
            }
            "sculpture" => {
                let s = media::sculpture_spec(r)?;
                let f = r.required_field(0, "the field to sculpt")?;
                rgb_to_rgba(&media::relief_preview(&media::heights(f, &s, seed), &s))
            }
            _ => return Err(Error::new("a sound has no picture", r.span)),
        })
    })
}

/// The samples (mono, 44.1 kHz, f32 little-endian) of a sound, or of a video's soundtrack.
#[unsafe(no_mangle)]
pub extern "C" fn alea_sound(index: usize) -> u32 {
    with_state(|st| {
        let seed = st.edition.seed;
        let r = render_at(st, index)?;
        let channels = match r.kind.as_str() {
            "sound" => media::sound_channels(r, seed)?,
            "video" => media::video_channels(r, &media::video_spec(r)?, seed)?,
            _ => return Err(Error::new("this render has no sound", r.span)),
        };
        // The number of channels, then each channel in turn, as f32.
        let mut out = (channels.len() as f32).to_le_bytes().to_vec();
        for c in &channels {
            out.extend(c.iter().flat_map(|x| (*x as f32).to_le_bytes()));
        }
        Ok(out)
    })
}

/// The music of an edition as a Standard MIDI File: render `index` if it is
/// a `render midi`, otherwise (index = any) what it would play live.
#[unsafe(no_mangle)]
pub extern "C" fn alea_midi(index: usize) -> u32 {
    with_state(|st| {
        let name = st.edition.name.clone();
        let score = match st.edition.renders.get(index) {
            Some(r) if r.kind == "midi" => media::midi_score(r, &name)?,
            _ => match media::live_score(&st.edition) {
                Some(s) => s?,
                None => return Err(Error::bare("this edition has no music to send: give it `render midi(…)` or sound layers")),
            },
        };
        Ok(crate::midi::smf(&score, &name))
    })
}

/// A score as timed messages, for the page to play (Web MIDI, or a preview):
/// the duration (f32), then for each message its time in seconds (f32), its
/// length, and up to three bytes.
#[unsafe(no_mangle)]
pub extern "C" fn alea_score(index: usize) -> u32 {
    with_state(|st| {
        let name = st.edition.name.clone();
        let score = match st.edition.renders.get(index) {
            Some(r) if r.kind == "midi" => media::midi_score(r, &name)?,
            _ => match media::live_score(&st.edition) {
                Some(s) => s?,
                None => return Err(Error::bare("this edition has no music")),
            },
        };
        let mut out = (score.duration as f32).to_le_bytes().to_vec();
        for m in &score.msgs {
            out.extend((m.at as f32).to_le_bytes());
            out.push(m.bytes.len() as u8);
            let mut b = [0u8; 3];
            b[..m.bytes.len().min(3)].copy_from_slice(&m.bytes[..m.bytes.len().min(3)]);
            out.extend(b);
        }
        Ok(out)
    })
}

/// A sculpture as binary STL.
#[unsafe(no_mangle)]
pub extern "C" fn alea_stl(index: usize) -> u32 {
    with_state(|st| {
        let seed = st.edition.seed;
        let r = render_at(st, index)?;
        let s = media::sculpture_spec(r)?;
        let f = r.required_field(0, "the field to sculpt")?;
        Ok(media::stl_bytes(&media::sculpture_mesh(&media::heights(f, &s, seed), &s), seed))
    })
}

/// An image at its full requested size, as PNG (for downloading).
#[unsafe(no_mangle)]
pub extern "C" fn alea_png(index: usize) -> u32 {
    with_state(|st| {
        let seed = st.edition.seed;
        let r = render_at(st, index)?;
        let s = media::image_spec(r)?;
        let rgb = r.visual("the field to paint")?.rgb(s.w, s.h, s.at, seed, &s.stops, s.range);
        let img = image::RgbImage::from_raw(s.w as u32, s.h as u32, rgb).expect("buffer size");
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|e| Error::bare(format!("encoding PNG: {e}")))?;
        Ok(png)
    })
}
