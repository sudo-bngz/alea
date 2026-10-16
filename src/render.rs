//! Native backends: write an edition's media to files (PNG, WAV, MP4/GIF via
//! ffmpeg, STL). The computation itself lives in `media.rs`, shared with the browser.

use std::collections::HashMap;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Res};
use crate::eval::{Edition, Render};
use crate::media::{self, SAMPLE_RATE};

pub fn render_all(ed: &Edition, dir: &Path, slug: &str) -> Res<Vec<(String, PathBuf)>> {
    let mut outputs = Vec::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for r in &ed.renders {
        let k = counts.entry(r.kind.as_str()).or_default();
        let suffix = if *k > 0 { format!("-{k}") } else { String::new() };
        *k += 1;
        let ext = match r.kind.as_str() {
            "image" => "png",
            "sound" => "wav",
            "video" if media::video_spec(r)?.gif => "gif",
            "video" => "mp4",
            _ => "stl",
        };
        let path = dir.join(format!("{slug}-{:016x}{suffix}.{ext}", ed.seed));
        match r.kind.as_str() {
            "image" => image(ed, r, &path)?,
            "sound" => sound(ed, r, &path)?,
            "video" => video(ed, r, &path)?,
            _ => sculpture(ed, r, &path)?,
        }
        outputs.push((r.kind.clone(), path));
    }
    Ok(outputs)
}

fn io_err(path: &Path, e: impl std::fmt::Display) -> Error {
    Error::bare(format!("writing {}: {e}", path.display()))
}

fn image(ed: &Edition, r: &Render, path: &Path) -> Res<()> {
    let s = media::image_spec(r)?;
    let rgb = r.visual("the field to paint")?.rgb(s.w, s.h, s.at, ed.seed, &s.stops, s.range);
    let img = image::RgbImage::from_raw(s.w as u32, s.h as u32, rgb).expect("buffer size");
    img.save(path).map_err(|e| io_err(path, e))
}

fn sound(ed: &Edition, r: &Render, path: &Path) -> Res<()> {
    write_wav(path, &media::sound_channels(r, ed.seed)?)
}

/// 16-bit WAV, mono or stereo (channels interleaved).
fn write_wav(path: &Path, channels: &[Vec<f64>]) -> Res<()> {
    let spec = hound::WavSpec {
        channels: channels.len() as u16,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut wav = hound::WavWriter::create(path, spec).map_err(|e| io_err(path, e))?;
    for s in 0..channels[0].len() {
        for c in channels {
            wav.write_sample((c[s] * i16::MAX as f64) as i16).map_err(|e| io_err(path, e))?;
        }
    }
    wav.finalize().map_err(|e| io_err(path, e))
}

fn sculpture(ed: &Edition, r: &Render, path: &Path) -> Res<()> {
    let s = media::sculpture_spec(r)?;
    let f = r.required_field(0, "the field to sculpt")?;
    let mesh = media::sculpture_mesh(&media::heights(f, &s, ed.seed), &s);
    std::fs::write(path, media::stl_bytes(&mesh, ed.seed)).map_err(|e| io_err(path, e))
}

/// `t` runs from 0 to 1 over the video. Frames are streamed straight into
/// ffmpeg (no temporary images). By default the video carries a soundtrack
/// synthesized from the same field — the same edition, seen and heard at once.
fn video(ed: &Edition, r: &Render, path: &Path) -> Res<()> {
    let s = media::video_spec(r)?;
    let visual = r.visual("the field to animate")?;
    let frames = ((s.duration * s.fps as f64).round() as usize).max(1);

    let wav = path.with_extension("audio.tmp.wav");
    if s.audio {
        write_wav(&wav, &media::video_channels(r, &s, ed.seed)?)?;
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24"])
        .args(["-s", &format!("{}x{}", s.w, s.h), "-r", &s.fps.to_string(), "-i", "-"]);
    if s.audio {
        cmd.arg("-i").arg(&wav);
    }
    if s.gif {
        cmd.args(["-vf", "split[a][b];[a]palettegen[p];[b][p]paletteuse", "-loop", "0"]);
    } else {
        cmd.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18", "-movflags", "+faststart"]);
        if s.audio {
            cmd.args(["-c:a", "aac", "-b:a", "192k", "-shortest"]);
        }
    }
    cmd.arg(path).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());

    let cleanup = || {
        let _ = std::fs::remove_file(&wav);
    };
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            cleanup();
            return Err(Error::new(
                "`video` needs ffmpeg, which was not found on this machine — install it from https://ffmpeg.org \
                 (or render an image or a sound instead)",
                r.span,
            ));
        }
        Err(e) => {
            cleanup();
            return Err(io_err(path, e));
        }
    };

    let progress = std::io::stderr().is_terminal();
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut broken = false;
    // A field: what doesn't move is computed once for the whole video, and
    // one colour scale serves every frame. A painting looks after its own.
    let (still, b) = match &visual {
        media::Visual::Field(f) => (
            Some(f.cached_for_frames(s.w, s.h, ed.seed, 48)),
            Some(s.range.unwrap_or_else(|| media::animation_bounds(f, s.w, s.h, ed.seed))),
        ),
        media::Visual::Paint(_) => (None, None),
    };
    for k in 0..frames {
        let t = k as f64 / frames as f64;
        let rgb = match &still {
            Some(f) => media::frame_rgb(f, s.w, s.h, t, ed.seed, &s.stops, b),
            None => visual.rgb(s.w, s.h, t, ed.seed, &s.stops, None),
        };
        if stdin.write_all(&rgb).is_err() {
            broken = true; // ffmpeg stopped early: its own message is reported below
            break;
        }
        if progress {
            eprint!("\r  video      frame {}/{frames}", k + 1);
        }
    }
    if progress {
        eprint!("\r\x1b[K");
    }
    drop(stdin);
    let mut log = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut log);
    }
    let status = child.wait().map_err(|e| io_err(path, e))?;
    cleanup();
    if broken || !status.success() {
        let tail: Vec<&str> = log.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        return Err(Error::bare(format!("ffmpeg failed while writing {}:\n  {}", path.display(), tail.join("\n  "))));
    }
    Ok(())
}
