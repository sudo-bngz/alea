//! A knob for a plugin parameter: drag up or down (Shift: finer), scroll,
//! double-click for its default. The host sees each gesture as one change,
//! so it records automation as usual.

use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, pos2, vec2, Color32, Response, Sense, Shape, Stroke, Ui};

const START: f32 = 0.75 * std::f32::consts::PI; // bottom left
const SWEEP: f32 = 1.5 * std::f32::consts::PI; // to bottom right, through the top

pub fn knob<P: Param>(ui: &mut Ui, param: &P, setter: &ParamSetter, label: &str, diameter: f32, accent: Color32) -> Response {
    let width = diameter.max(86.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, diameter + 36.0), Sense::click_and_drag());
    let mut value = param.unmodulated_normalized_value();

    if response.drag_started() {
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        let fine = ui.input(|i| i.modifiers.shift);
        let delta = -response.drag_delta().y * if fine { 0.0008 } else { 0.005 };
        value = (value + delta).clamp(0.0, 1.0);
        setter.set_parameter_normalized(param, value);
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if response.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter_normalized(param, param.default_normalized_value());
        setter.end_set_parameter(param);
        value = param.default_normalized_value();
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            value = (value + scroll * 0.002).clamp(0.0, 1.0);
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, value);
            setter.end_set_parameter(param);
        }
    }

    let painter = ui.painter_at(rect);
    let radius = diameter * 0.5 - 3.0;
    let center = pos2(rect.center().x, rect.top() + diameter * 0.5);
    let point = |angle: f32, r: f32| center + vec2(angle.cos(), angle.sin()) * r;
    let arc = |from: f32, to: f32| -> Vec<egui::Pos2> {
        let steps = 48;
        (0..=steps).map(|k| point(from + (to - from) * k as f32 / steps as f32, radius)).collect()
    };
    let active = response.hovered() || response.dragged();
    painter.circle_filled(center, radius - 4.0, if active { Color32::from_rgb(250, 250, 252) } else { Color32::WHITE });
    painter.circle_stroke(center, radius - 4.0, Stroke::new(1.0_f32, Color32::from_rgb(210, 213, 222)));
    painter.add(Shape::line(arc(START, START + SWEEP), Stroke::new(3.0_f32, Color32::from_rgb(220, 223, 230))));
    if value > 0.001 {
        painter.add(Shape::line(arc(START, START + SWEEP * value), Stroke::new(3.0_f32, accent)));
    }
    let angle = START + SWEEP * value;
    painter.line_segment([point(angle, radius * 0.25), point(angle, radius - 7.0)], Stroke::new(2.0_f32, Color32::from_rgb(24, 27, 48)));

    let text = Color32::from_rgb(24, 27, 48);
    let muted = Color32::from_rgb(91, 96, 128);
    painter.text(
        pos2(rect.center().x, rect.top() + diameter + 4.0),
        egui::Align2::CENTER_TOP,
        label,
        egui::FontId::proportional(12.0),
        text,
    );
    painter.text(
        pos2(rect.center().x, rect.top() + diameter + 19.0),
        egui::Align2::CENTER_TOP,
        param.to_string(),
        egui::FontId::monospace(11.0),
        muted,
    );
    response.on_hover_text("drag up or down (Shift: finer) · scroll · double-click: default")
}
