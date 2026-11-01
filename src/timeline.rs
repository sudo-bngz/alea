//! Timelines: scenes one after another, with transitions between them.
//!
//! `timeline([scene(a, 4), glitch(0.5), scene(b, 6), fade(1), …])` plays
//! each scene for its seconds, each with its own `t` running from 0 to 1,
//! and crosses from one to the next with a transition that straddles the
//! cut: half of it on the scene that ends, half on the one that starts. With
//! `loop: true` the last transition leads back to the first scene, so a
//! video of the timeline's length loops without a seam.
//!
//! A transition is the same for a field and for a painting: at every point it
//! says where to look in the scene that ends (A) and in the one that starts
//! (B), and how much of each to take (`taps`). A field evaluates A and B
//! there; a painting reads the pixels of A's and B's frames there.

use std::sync::{Arc, Mutex};

use crate::eval::hash;

/// How one scene crosses into the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Cut,
    /// A crossfade, or a fade through a flat value (0 black, 1 white).
    Fade { through: Option<f64> },
    /// A line sweeps across, at that angle (radians), with a soft edge.
    Wipe { angle: f64, softness: f64 },
    /// The scene that ends is pushed out, the next pushed in, at that angle.
    Slide { angle: f64 },
    /// Slices torn sideways, blocks out of place, bursts of static.
    Glitch { amount: f64 },
    /// The two scenes melt into each other through a noise field.
    Morph { amount: f64 },
    /// The picture breaks into blocks that grow, change, and shrink back.
    Pixels { size: f64 },
}

impl Kind {
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Cut => "jump",
            Kind::Fade { .. } => "fade",
            Kind::Wipe { .. } => "wipe",
            Kind::Slide { .. } => "slide",
            Kind::Glitch { .. } => "glitch",
            Kind::Morph { .. } => "morph",
            Kind::Pixels { .. } => "pixels",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transition {
    pub kind: Kind,
    pub seconds: f64,
}

#[derive(Clone, Debug)]
pub struct Scene<S> {
    pub source: S,
    pub seconds: f64,
    /// The scene's own `t` runs from `from` to `to` over its seconds.
    pub from: f64,
    pub to: f64,
}

#[derive(Clone, Debug)]
pub struct Timeline<S> {
    pub scenes: Vec<Scene<S>>,
    /// Between scene k and k + 1 (and, looping, between the last and the first).
    pub transitions: Vec<Transition>,
    pub looping: bool,
    pub seed: u64,
}

/// Where a moment falls: in one scene, or crossing from one to the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Moment {
    Scene { index: usize, t: f64 },
    /// From scene `a` (at its time `ta`) to scene `b` (at `tb`), `p` of the way.
    Cross { a: usize, ta: f64, b: usize, tb: f64, p: f64, kind: Kind },
}

impl<S> Timeline<S> {
    pub fn length(&self) -> f64 {
        self.scenes.iter().map(|s| s.seconds).sum()
    }

    /// When scene k starts, in seconds.
    fn start(&self, k: usize) -> f64 {
        self.scenes[..k].iter().map(|s| s.seconds).sum()
    }

    /// The scene's own time at `s` seconds into the timeline (clamped to its ends).
    fn local(&self, k: usize, s: f64) -> f64 {
        let sc = &self.scenes[k];
        let u = ((s - self.start(k)) / sc.seconds.max(1e-9)).clamp(0.0, 1.0);
        sc.from + (sc.to - sc.from) * u
    }

    /// The cuts: (seconds, from scene, to scene, transition), in order.
    pub fn cuts(&self) -> Vec<(f64, usize, usize, Transition)> {
        let n = self.scenes.len();
        let mut out = Vec::new();
        for (k, tr) in self.transitions.iter().enumerate() {
            if k + 1 < n {
                out.push((self.start(k + 1), k, k + 1, *tr));
            } else if self.looping && n > 1 {
                out.push((self.length(), n - 1, 0, *tr));
            }
        }
        out
    }

    /// What plays at `t` (0 to 1 over the whole timeline).
    pub fn at(&self, t: f64) -> Moment {
        let total = self.length();
        let s = (t * total).rem_euclid(total.max(1e-9));
        for (cut, a, b, tr) in self.cuts() {
            if tr.kind == Kind::Cut || tr.seconds <= 0.0 {
                continue;
            }
            let half = tr.seconds / 2.0;
            // the last cut, looping, straddles the end and the start
            let d = if b == 0 && a + 1 == self.scenes.len() && self.looping {
                if s >= cut - half { s - cut } else if s < half { s } else { continue }
            } else {
                s - cut
            };
            if d >= -half && d < half {
                let p = (d + half) / tr.seconds;
                let ta = self.local(a, cut + d);
                let tb = self.local(b, if b == 0 && a != 0 && a + 1 == self.scenes.len() { d.max(0.0) } else { cut + d });
                return Moment::Cross { a, ta, b, tb, p, kind: tr.kind };
            }
        }
        let mut k = 0;
        while k + 1 < self.scenes.len() && s >= self.start(k + 1) {
            k += 1;
        }
        Moment::Scene { index: k, t: self.local(k, s) }
    }

    /// Which scene plays along the timeline, as a curve of t: 0, 1, 2…
    pub fn index_curve(&self) -> Vec<f32> {
        let n = 2000;
        (0..n)
            .map(|k| match self.at(k as f64 / (n - 1) as f64) {
                Moment::Scene { index, .. } => index as f32,
                Moment::Cross { a, b, p, .. } => if p < 0.5 { a as f32 } else { b as f32 },
            })
            .collect()
    }
}

/// Where to read, and how much to take, for one point of a transition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    A,
    B,
    Flat(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tap {
    pub source: Source,
    pub x: f64,
    pub y: f64,
    pub weight: f64,
    /// For paintings: red read that far to one side, blue to the other.
    pub chroma: f64,
}

fn tap(source: Source, x: f64, y: f64, weight: f64) -> Tap {
    Tap { source, x, y, weight, chroma: 0.0 }
}

fn unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

fn smooth(a: f64, b: f64, v: f64) -> f64 {
    if b <= a {
        return if v >= b { 1.0 } else { 0.0 };
    }
    let u = ((v - a) / (b - a)).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// The taps of a transition `kind` at point (x, y), `p` of the way (0 to 1),
/// at `clock` seconds into the timeline (for what changes frame to frame).
/// At most three taps; their weights sum to 1.
pub fn taps(kind: Kind, p: f64, x: f64, y: f64, clock: f64, seed: u64) -> Vec<Tap> {
    match kind {
        Kind::Cut => vec![tap(if p < 0.5 { Source::A } else { Source::B }, x, y, 1.0)],
        Kind::Fade { through: None } => vec![tap(Source::A, x, y, 1.0 - p), tap(Source::B, x, y, p)],
        Kind::Fade { through: Some(v) } => {
            if p < 0.5 {
                let m = p * 2.0;
                vec![tap(Source::A, x, y, 1.0 - m), tap(Source::Flat(v), x, y, m)]
            } else {
                let m = (p - 0.5) * 2.0;
                vec![tap(Source::Flat(v), x, y, 1.0 - m), tap(Source::B, x, y, m)]
            }
        }
        Kind::Wipe { angle, softness } => {
            // along the direction, from −0.75 to 0.75 over the canvas and a little beyond
            let c = (x - 0.5) * angle.cos() + (y - 0.5) * angle.sin();
            let edge = -0.75 - softness + (1.5 + 2.0 * softness) * p;
            let m = 1.0 - smooth(edge - softness, edge + softness, c);
            vec![tap(Source::A, x, y, 1.0 - m), tap(Source::B, x, y, m)]
        }
        Kind::Slide { angle } => {
            let (dx, dy) = (angle.cos(), angle.sin());
            // the canvas spans `e` along the direction; c runs from −0.5 to 0.5 across it
            let e = dx.abs() + dy.abs();
            let c = ((x - 0.5) * dx + (y - 0.5) * dy) / e;
            // A has moved `p` of the way out; B is still 1 − p of the way behind
            if c + p < 0.5 + 1e-9 {
                vec![tap(Source::A, x + p * e * dx, y + p * e * dy, 1.0)]
            } else {
                vec![tap(Source::B, x - (1.0 - p) * e * dx, y - (1.0 - p) * e * dy, 1.0)]
            }
        }
        Kind::Pixels { size } => {
            let s = (size * (std::f64::consts::PI * p).sin()).max(1e-6);
            let bx = (x / s).floor() * s + s / 2.0;
            let by = (y / s).floor() * s + s / 2.0;
            vec![tap(if p < 0.5 { Source::A } else { Source::B }, bx, by, 1.0)]
        }
        Kind::Morph { amount } => {
            let n = |u: f64, v: f64, salt: u64| crate::eval::noise_at(u, v, 0.0, seed ^ salt);
            let swell = amount * (std::f64::consts::PI * p).sin();
            let (fx, fy) = (n(x * 3.0, y * 3.0, 0x11), n(x * 3.0 + 7.0, y * 3.0, 0x22));
            // the two scenes drift along the same flow, A away, B in; the mix
            // is decided point by point by a noise, so B seeps in
            let d = 0.5 + 0.5 * n(x * 2.5, y * 2.5, 0x33);
            let m = ((p - d * 0.6) / 0.4).clamp(0.0, 1.0);
            vec![
                tap(Source::A, x + fx * swell, y + fy * swell, 1.0 - m),
                tap(Source::B, x - fx * swell, y - fy * swell, m),
            ]
        }
        Kind::Glitch { amount } => {
            // what is torn changes 24 times a second
            let tick = (clock * 24.0).floor() as i64 as u64;
            let force = amount * (std::f64::consts::PI * p).sin();
            let h = |salt: u64, a: u64, b: u64| unit(hash(&[seed ^ 0x6117, salt, tick, a, b]));
            // the picture is cut into slices of uneven height; a slice is torn
            // sideways, and shows A or B, more often B as p grows
            let rows = 18.0 + 30.0 * h(1, 0, 0);
            let slice = (y * rows + h(2, 0, 0) * rows).floor() as i64 as u64;
            let torn = h(3, slice, 0) < force * 0.9;
            let which = if h(4, slice, 1) < p { Source::B } else { Source::A };
            let shift = if torn { (h(5, slice, 2) - 0.5) * 0.3 * force * (1.0 + h(6, slice, 3)) } else { 0.0 };
            // some blocks of a coarse grid jump elsewhere; some burst into static
            let (gx, gy) = ((x * 9.0).floor() as i64 as u64, (y * 16.0).floor() as i64 as u64);
            let block = h(7, gx, gy);
            if block < force * 0.08 {
                let v = unit(hash(&[seed ^ 0x57A7, tick, (x * 400.0) as i64 as u64, (y * 700.0) as i64 as u64]));
                return vec![tap(Source::Flat(v), x, y, 1.0)];
            }
            let (jx, jy) = if block < force * 0.3 { ((h(8, gx, gy) - 0.5) * 0.5 * force, (h(9, gx, gy) - 0.5) * 0.25 * force) } else { (0.0, 0.0) };
            let mut t = tap(which, x + shift + jx, y + jy, 1.0);
            t.chroma = if torn || block < force * 0.3 { 0.012 * force * (1.0 + 2.0 * h(10, slice, 4)) } else { 0.0 };
            vec![t]
        }
    }
}

// ── the default length of a video or a sound: the last timeline built ──

static HINT: Mutex<Option<f64>> = Mutex::new(None);

pub fn set_hint(seconds: f64) {
    *HINT.lock().unwrap() = Some(seconds);
}

pub fn hint() -> Option<f64> {
    *HINT.lock().unwrap()
}

pub fn reset_hint() {
    *HINT.lock().unwrap() = None;
}

/// A timeline shared between the field and the painting worlds.
pub type Shared<S> = Arc<Timeline<S>>;

#[cfg(test)]
mod tests {
    use super::*;

    fn tl(secs: &[f64], trans: &[(Kind, f64)], looping: bool) -> Timeline<usize> {
        Timeline {
            scenes: secs.iter().enumerate().map(|(k, s)| Scene { source: k, seconds: *s, from: 0.0, to: 1.0 }).collect(),
            transitions: trans.iter().map(|(k, s)| Transition { kind: *k, seconds: *s }).collect(),
            looping,
            seed: 7,
        }
    }

    #[test]
    fn scenes_follow_each_other_and_keep_their_own_time() {
        let t = tl(&[4.0, 6.0], &[(Kind::Cut, 0.0)], false);
        assert_eq!(t.length(), 10.0);
        assert_eq!(t.at(0.0), Moment::Scene { index: 0, t: 0.0 });
        assert_eq!(t.at(0.2), Moment::Scene { index: 0, t: 0.5 });
        assert_eq!(t.at(0.4), Moment::Scene { index: 1, t: 0.0 });
        assert_eq!(t.at(0.7), Moment::Scene { index: 1, t: 0.5 });
        assert_eq!(t.cuts().len(), 1);
    }

    #[test]
    fn a_transition_straddles_the_cut() {
        let t = tl(&[4.0, 6.0], &[(Kind::Fade { through: None }, 2.0)], false);
        // from 3 s to 5 s: p from 0 to 1
        assert!(matches!(t.at(0.29), Moment::Scene { index: 0, .. }));
        match t.at(0.4) {
            Moment::Cross { a, b, p, ta, tb, .. } => {
                assert_eq!((a, b), (0, 1));
                assert!((p - 0.5).abs() < 1e-9);
                assert!((ta - 1.0).abs() < 1e-9 && tb.abs() < 1e-9, "at the cut: A ends, B starts");
            }
            m => panic!("{m:?}"),
        }
        assert!(matches!(t.at(0.51), Moment::Scene { index: 1, .. }));
    }

    #[test]
    fn looping_crosses_the_end() {
        let t = tl(&[5.0, 5.0], &[(Kind::Cut, 0.0), (Kind::Fade { through: None }, 2.0)], true);
        assert_eq!(t.cuts().len(), 2);
        match t.at(0.95) {
            Moment::Cross { a, b, p, .. } => {
                assert_eq!((a, b), (1, 0));
                assert!((p - 0.25).abs() < 1e-9);
            }
            m => panic!("{m:?}"),
        }
        match t.at(0.05) {
            Moment::Cross { a, b, p, tb, .. } => {
                assert_eq!((a, b), (1, 0));
                assert!((p - 0.75).abs() < 1e-9 && (tb - 0.1).abs() < 1e-9);
            }
            m => panic!("{m:?}"),
        }
        let unlooped = tl(&[5.0, 5.0], &[(Kind::Cut, 0.0), (Kind::Fade { through: None }, 2.0)], false);
        assert!(matches!(unlooped.at(0.95), Moment::Scene { index: 1, .. }), "no loop: the last transition is dropped");
    }

    #[test]
    fn taps_weigh_one() {
        let kinds = [
            Kind::Fade { through: None },
            Kind::Fade { through: Some(0.0) },
            Kind::Wipe { angle: 0.3, softness: 0.05 },
            Kind::Slide { angle: 1.2 },
            Kind::Glitch { amount: 0.8 },
            Kind::Morph { amount: 0.2 },
            Kind::Pixels { size: 0.1 },
        ];
        for kind in kinds {
            for k in 0..50 {
                let p = k as f64 / 49.0;
                let (x, y) = (0.137 + 0.7 * p, 0.8 - 0.6 * p);
                let ts = taps(kind, p, x, y, p * 3.0, 9);
                let w: f64 = ts.iter().map(|t| t.weight).sum();
                assert!((w - 1.0).abs() < 1e-9, "{kind:?} at {p}: {ts:?}");
                assert!(ts.iter().all(|t| t.weight >= 0.0));
            }
            // ends: all A at the start, all B at the end
            let start = taps(kind, 0.0, 0.3, 0.3, 0.0, 9);
            let end = taps(kind, 1.0, 0.3, 0.3, 5.0, 9);
            let only = |ts: &[Tap], s: Source| ts.iter().filter(|t| t.weight > 1e-9).all(|t| t.source == s);
            assert!(only(&start, Source::A), "{kind:?} starts on A: {start:?}");
            assert!(only(&end, Source::B), "{kind:?} ends on B: {end:?}");
        }
    }
}
