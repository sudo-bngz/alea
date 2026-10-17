//! The plugin's window: the work, its edition, its knobs, its notes and its
//! picture, moving with the music.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Stroke, TextureHandle, TextureOptions, Vec2};
use nih_plug_egui::{create_egui_editor, resizable_window::ResizableWindow};

use crate::knob::knob;

use crate::engine::{Shared, LAYERS, MACROS};
use crate::AleaParams;

struct Ui {
    path: String,
    picture: Option<TextureHandle>,
    frame: u64,
    show_error: bool,
}

const INK: Color32 = Color32::from_rgb(24, 27, 48);
const PAPER: Color32 = Color32::from_rgb(237, 239, 243);
const ACCENT: Color32 = Color32::from_rgb(217, 43, 106);
const MUTED: Color32 = Color32::from_rgb(91, 96, 128);
const ACTIVE: Color32 = Color32::from_rgb(255, 196, 61);
const SOLO: Color32 = Color32::from_rgb(64, 132, 230);
const LANES: [Color32; 5] = [
    Color32::from_rgb(217, 43, 106),
    Color32::from_rgb(55, 65, 196),
    Color32::from_rgb(7, 112, 95),
    Color32::from_rgb(150, 80, 0),
    Color32::from_rgb(125, 51, 170),
];

pub fn create(params: Arc<AleaParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let path = params.path.read().unwrap().clone();
    create_egui_editor(
        params.editor_state.clone(),
        Ui { path, picture: None, frame: 0, show_error: true },
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::light();
            style.visuals.panel_fill = PAPER;
            style.visuals.override_text_color = Some(INK);
            ctx.set_style(style);
        },
        move |ctx, setter, ui_state| {
            shared.window_open.store(true, Ordering::Relaxed);
            if ui_state.path.is_empty() {
                let p = params.path.read().unwrap();
                if !p.is_empty() {
                    ui_state.path = p.clone();
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            // linked: this window's knobs show what the group plays
            if params.link.value() {
                if let Some(g) = *shared.group.lock().unwrap() {
                    let set_i = |p: &IntParam, v: i32| {
                        if p.value() != v {
                            setter.begin_set_parameter(p);
                            setter.set_parameter(p, v);
                            setter.end_set_parameter(p);
                        }
                    };
                    set_i(&params.seed, g.seed);
                    set_i(&params.every, g.every);
                    if params.macros_on.value() != g.macros_on {
                        setter.begin_set_parameter(&params.macros_on);
                        setter.set_parameter(&params.macros_on, g.macros_on);
                        setter.end_set_parameter(&params.macros_on);
                    }
                    for k in 0..MACROS {
                        let p = &params.macros[k].value;
                        if (p.value() - g.macros[k]).abs() > 1e-6 {
                            setter.begin_set_parameter(p);
                            setter.set_parameter(p, g.macros[k]);
                            setter.end_set_parameter(p);
                        }
                    }
                }
            }
            let status = shared.status.lock().unwrap().clone();
            let seed = shared.now_seed.load(Ordering::Relaxed);
            let beat = f64::from_bits(shared.now_beat.load(Ordering::Relaxed));
            let edition = shared.editions.lock().unwrap().get(&seed).cloned();

            // the picture, when the painter has a new one
            if let Some((w, h, px, frame)) = shared.picture.lock().unwrap().as_ref() {
                if *frame != ui_state.frame {
                    ui_state.frame = *frame;
                    let image = egui::ColorImage::from_rgb([*w, *h], px);
                    match &mut ui_state.picture {
                        Some(t) => t.set(image, TextureOptions::LINEAR),
                        None => ui_state.picture = Some(ctx.load_texture("alea-picture", image, TextureOptions::LINEAR)),
                    }
                }
            }

            let egui_state = params.editor_state.clone();
            {
              ResizableWindow::new("alea-window").min_size(Vec2::new(640.0, 420.0)).show(ctx, egui_state.as_ref(), |ui| {
               egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| { egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 18, top: 8, bottom: 10 }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("~").size(22.0).color(ACCENT).strong());
                    ui.label(RichText::new("alea").size(20.0).strong());
                    ui.add_space(10.0);
                    let work = if status.work.is_empty() { "no work yet".to_string() } else { status.work.clone() };
                    ui.label(RichText::new(work).size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("edition 0x{seed:x}")).monospace().color(ACCENT));
                    });
                });
                ui.separator();

                // the work's file
                ui.horizontal(|ui| {
                    ui.label("Work");
                    let width = (ui.available_width() - 120.0).max(160.0);
                    let edit = ui.add(egui::TextEdit::singleline(&mut ui_state.path).desired_width(width).hint_text("/path/to/work.alea"));
                    let mut commit = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Open…").clicked() {
                        crate::engine::choose_file(shared.clone());
                    }
                    if let Some(p) = shared.picked.lock().unwrap().take() {
                        ui_state.path = p;
                        commit = true;
                    }
                    if ui.button("Use").clicked() {
                        commit = true;
                    }
                    if commit {
                        *params.path.write().unwrap() = ui_state.path.clone();
                        *shared.path.lock().unwrap() = ui_state.path.clone();
                    }
                });
                ui.label(RichText::new(format!("{}  ·  saved changes to the file are picked up while it plays", status.message)).color(MUTED).small());
                if let Some(e) = &status.error {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("error").color(ACCENT).strong());
                        if ui.small_button(if ui_state.show_error { "hide" } else { "show" }).clicked() {
                            ui_state.show_error = !ui_state.show_error;
                        }
                    });
                    if ui_state.show_error {
                        egui::ScrollArea::vertical().id_salt("error").max_height(90.0).show(ui, |ui| {
                            ui.label(RichText::new(e).monospace().size(11.0));
                        });
                    }
                }
                ui.add_space(6.0);

                // knobs on the left; the picture and the notes on the right
                let side = 150.0;
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            knob(ui, &params.seed, setter, "Edition", 64.0, ACCENT);
                            knob(ui, &params.every, setter, "New every", 64.0, LANES[1]);
                            if params.ports.value() {
                                knob(ui, &params.port_timing, setter, "Port timing", 46.0, SOLO);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("New edition").clicked() {
                                let fresh = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 65536) as i32;
                                setter.begin_set_parameter(&params.seed);
                                setter.set_parameter(&params.seed, fresh);
                                setter.end_set_parameter(&params.seed);
                            }
                        });
                        ui.horizontal(|ui| {
                            let toggle = |ui: &mut egui::Ui, p: &BoolParam, text: &str, tip: &str| {
                                if ui.selectable_label(p.value(), text).on_hover_text(tip).clicked() {
                                    setter.begin_set_parameter(p);
                                    setter.set_parameter(p, !p.value());
                                    setter.end_set_parameter(p);
                                }
                            };
                            toggle(ui, &params.link, "Link", "on: every alea on this work plays the same edition");
                            toggle(ui, &params.macros_on, "Macros", "on: the macro knobs drive the work's parameters");
                        });
                        ui.horizontal(|ui| {
                            let on = params.ports.value();
                            if ui.selectable_label(on, "Layers to tracks").on_hover_text("send each layer on its own channel of alea's MIDI port, for one track per layer in Live").clicked() {
                                setter.begin_set_parameter(&params.ports);
                                setter.set_parameter(&params.ports, !on);
                                setter.end_set_parameter(&params.ports);
                            }
                        });
                    });
                    ui.add_space(8.0);
                    match &ui_state.picture {
                        Some(t) => {
                            ui.image((t.id(), Vec2::splat(side)));
                        }
                        None => {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
                            ui.painter().rect_filled(r, 4.0, Color32::from_rgb(220, 223, 230));
                            let note = shared.picture_note.lock().unwrap().clone();
                            let text = if note.is_empty() { "the picture\nappears here".to_string() } else { note };
                            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(11.0), MUTED);
                        }
                    }
                    let width = ui.available_width().max(160.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, side), egui::Sense::hover());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, 4.0, Color32::WHITE);
                    if let Some(ed) = &edition {
                        if !ed.roll.is_empty() && ed.beats > 0.0 {
                            let lo = ed.roll.iter().map(|n| n.note).min().unwrap_or(0) as f32 - 1.0;
                            let hi = ed.roll.iter().map(|n| n.note).max().unwrap_or(127) as f32 + 1.0;
                            let row = rect.height() / (hi - lo + 1.0);
                            let x = |b: f64| rect.left() + (b / ed.beats) as f32 * rect.width();
                            let mut b = 0.0;
                            while b < ed.beats {
                                p.line_segment([egui::pos2(x(b), rect.top()), egui::pos2(x(b), rect.bottom())], Stroke::new(1.0_f32, Color32::from_rgb(232, 234, 240)));
                                b += 4.0;
                            }
                            let on: [bool; LAYERS] = std::array::from_fn(|k| params.layers[k].on.value());
                            let heard = crate::engine::layers(&on, params.layer.value());
                            for n in &ed.roll {
                                let y = rect.bottom() - (n.note as f32 - lo + 1.0) * row;
                                let shown = n.track >= 64 || heard & (1u64 << n.track) != 0;
                                let strength = if shown { 0.35 + 0.65 * n.velocity as f32 / 127.0 } else { 0.08 };
                                let c = LANES[n.track as usize % LANES.len()].gamma_multiply(strength);
                                let r = egui::Rect::from_min_max(egui::pos2(x(n.start), y), egui::pos2(x(n.end).max(x(n.start) + 2.0), y + row.max(2.0) - 0.5));
                                p.rect_filled(r, 1.0, c);
                            }
                            let px = x(beat);
                            p.line_segment([egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())], Stroke::new(1.5_f32, ACCENT));
                        }
                    } else {
                        p.text(rect.center(), egui::Align2::CENTER_CENTER, "the notes of the edition appear here", egui::FontId::proportional(13.0), MUTED);
                    }
                });
                ui.add_space(6.0);

                // the layers, as in a mixer: a switch and a solo for each
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Layers").strong());
                    let solo = params.layer.value();
                    let set_solo = |v: i32| {
                        setter.begin_set_parameter(&params.layer);
                        setter.set_parameter(&params.layer, v);
                        setter.end_set_parameter(&params.layer);
                    };
                    let set_on = |k: usize, v: bool| {
                        let p = &params.layers[k].on;
                        if p.value() != v {
                            setter.begin_set_parameter(p);
                            setter.set_parameter(p, v);
                            setter.end_set_parameter(p);
                        }
                    };
                    if ui.button("all on").on_hover_text("every layer on, no solo").clicked() {
                        set_solo(0);
                        (0..LAYERS).for_each(|k| set_on(k, true));
                    }
                    if let Some(ed) = &edition {
                        for (k, name) in ed.tracks.iter().enumerate() {
                            let on = k >= LAYERS || params.layers[k].on.value();
                            let soloed = solo == k as i32 + 1;
                            let heard = on && (solo == 0 || soloed);
                            // one chip, kept whole: to the next row if it does not fit
                            let text = ui.fonts(|f| f.layout_no_wrap(name.clone(), egui::FontId::proportional(13.0), INK).size().x);
                            let width = text + if params.ports.value() { 112.0 } else { 78.0 };
                            if ui.available_width() < width {
                                ui.end_row();
                            }
                            ui.allocate_ui_with_layout(Vec2::new(width, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            egui::Frame::new()
                                .fill(if heard { Color32::WHITE } else { Color32::from_rgb(226, 228, 234) })
                                .stroke(Stroke::new(1.0_f32, LANES[k % LANES.len()].gamma_multiply(if heard { 1.0 } else { 0.35 })))
                                .corner_radius(4.0)
                                .inner_margin(egui::Margin::symmetric(4, 2))
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    // the switch: on (amber) or off, as Live's track activator
                                    let number = RichText::new(format!("{}", k + 1)).monospace().strong().color(if on { INK } else { MUTED });
                                    let switch = egui::Button::new(number)
                                        .fill(if on { ACTIVE } else { Color32::from_rgb(200, 203, 212) })
                                        .min_size(Vec2::new(22.0, 18.0));
                                    let r = ui.add_enabled(k < LAYERS, switch).on_hover_text(if k < LAYERS {
                                        "switch this layer on or off (Cmd/Ctrl-click: only this one)"
                                    } else {
                                        "always on (only the first 16 layers have a switch)"
                                    });
                                    if r.clicked() {
                                        if ui.input(|i| i.modifiers.command) {
                                            (0..LAYERS).for_each(|j| set_on(j, j == k));
                                        } else {
                                            set_on(k, !on);
                                        }
                                    }
                                    ui.label(RichText::new(name).color(if heard { INK } else { MUTED }));
                                    if params.ports.value() {
                                        ui.label(RichText::new(format!("ch {}", k % 16 + 1)).small().monospace().color(SOLO));
                                    }
                                    let s = egui::Button::new(RichText::new("S").small().strong().color(if soloed { Color32::WHITE } else { MUTED }))
                                        .fill(if soloed { SOLO } else { Color32::TRANSPARENT })
                                        .min_size(Vec2::new(18.0, 18.0));
                                    if ui.add(s).on_hover_text("solo: send only this layer").clicked() {
                                        set_solo(if soloed { 0 } else { k as i32 + 1 });
                                    }
                                });
                            });
                        }
                    }
                });

                if params.ports.value() {
                    let port = &shared.port;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Send to").strong());
                        let current = port.target.lock().unwrap().clone();
                        let shown = if current.is_empty() { "alea (its own port)".to_string() } else { current.clone() };
                        let mut chosen: Option<String> = None;
                        let combo = egui::ComboBox::from_id_salt("port-target").selected_text(shown).width(220.0).show_ui(ui, |ui| {
                            if ui.selectable_label(current.is_empty(), "alea (its own port)").clicked() {
                                chosen = Some(String::new());
                            }
                            for name in port.outputs.lock().unwrap().iter() {
                                if ui.selectable_label(*name == current, name).clicked() {
                                    chosen = Some(name.clone());
                                }
                            }
                        });
                        if combo.response.clicked() {
                            port.list.store(true, Ordering::Relaxed);
                        }
                        if let Some(c) = chosen {
                            *params.port_target.write().unwrap() = c.clone();
                            *port.target.lock().unwrap() = c;
                        }
                        if ui.button("Test").on_hover_text("a C on every channel, now: each track listening to the port should play it").clicked() {
                            port.test.store(true, Ordering::Relaxed);
                        }
                        let (q, sent, refused) = (port.queued.load(Ordering::Relaxed), port.sent.load(Ordering::Relaxed), port.refused.load(Ordering::Relaxed));
                        let mut counts = format!("queued {q} · sent {sent}");
                        if refused > 0 {
                            counts.push_str(&format!(" · refused {refused} ({})", port.last_error.lock().unwrap()));
                        }
                        ui.label(RichText::new(counts).monospace().small().color(MUTED));
                    });
                    let status = port.status.lock().unwrap().clone();
                    ui.label(RichText::new(if status.is_empty() { "opening the port…".to_string() } else { status }).color(SOLO).small());
                    ui.label(RichText::new(crate::ports::HOW).color(MUTED).small());
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                }

                // the macros, as a row of knobs named after the work's parameters
                if params.macros_on.value() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for k in 0..MACROS {
                            let label = status.macro_names.get(k).cloned().unwrap_or_else(|| "—".into());
                            knob(ui, &params.macros[k].value, setter, &label, 46.0, LANES[2]);
                        }
                    });
                }

                ui.add_space(6.0);
                if let Some(ed) = &edition {
                    ui.label(
                        RichText::new(format!("{}  ·  {:.0} bars at the work's {:.0} BPM (it plays at Live's tempo)", ed.name, ed.beats / 4.0, ed.tempo))
                            .color(MUTED)
                            .small(),
                    );
                }
                ui.label(
                    RichText::new("Switch layers on and off as they play, or solo one. One track per layer: turn on Layers to tracks, and give each instrument track the channel of its layer. Or, with Layers to tracks off: a MIDI track with \"MIDI From\" this track and alea, Monitor In.")
                        .color(MUTED)
                        .small(),
                );
               }); });
              });
            }
        },
    )
}
