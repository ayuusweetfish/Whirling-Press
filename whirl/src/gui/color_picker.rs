// Copied and edited from egui-0.36.2: egui/widgets/color_picker.rs
// Copyright (c) 2018-2021 Emil Ernerfeldt <emil.ernerfeldt@gmail.com>

// Changes from the original: this implementation one operates entirely in sRGB space;
// Sliders and numeric fields are all in gamma-encoded floating-point sRGB components.

//! Color picker widgets.

use crate::f;

use egui::{
    Context, DragValue, Id, Painter, Popup, PopupCloseBehavior, Response, Sense, Ui,
    WidgetInfo, Role, epaint, lerp, remap_clamp,
};
use egui::epaint::{
    Mesh, Rect, Shape, Stroke, StrokeKind, Vec2,
    ecolor::{Color32, Rgba, hsv_from_rgb, rgb_from_hsv},
    pos2, vec2,
};

// Like `ecolor::Hsva`, but with gamma-encoded components
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HsvaSrgb {
    h: f32,
    s: f32,
    v: f32,
    a: f32,
}

impl HsvaSrgb {
    fn from_rgba_unmultiplied([r, g, b, a]: [f32; 4]) -> Self {
        let (h, s, v) = hsv_from_rgb([r, g, b]);
        Self { h, s, v, a }
    }

    fn to_rgba_unmultiplied(self) -> [f32; 4] {
        let [r, g, b] = rgb_from_hsv((self.h, self.s, self.v));
        [r, g, b, self.a]
    }
}

impl From<HsvaSrgb> for Color32 {
    fn from(hsva: HsvaSrgb) -> Self {
        let [r, g, b] = rgb_from_hsv((hsva.h, hsva.s, hsva.v));
        fn quant_u8(x: f32) -> u8 {
            (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
        }
        Color32::from_rgba_unmultiplied(
            quant_u8(r),
            quant_u8(g),
            quant_u8(b),
            quant_u8(hsva.a.abs()),
        )
    }
}

fn contrast_color(color: impl Into<Rgba>) -> Color32 {
    if color.into().intensity() < 0.5 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

/// Number of vertices per dimension in the color sliders.
/// We need at least 6 for hues, and more for smooth 2D areas.
/// Should always be a multiple of 6 to hit the peak hues in HSV/HSL (every 60°).
const N: u32 = 6 * 6;

fn background_checkers(painter: &Painter, rect: Rect) {
    let rect = rect.shrink(0.5); // Small hack to avoid the checkers from peeking through the sides
    if !rect.is_positive() {
        return;
    }

    let dark_color = Color32::from_gray(32);
    let bright_color = Color32::from_gray(128);

    let checker_size = Vec2::splat(rect.height() / 2.0);
    let n = (rect.width() / checker_size.x).round() as u32;

    let mut mesh = Mesh::default();
    mesh.add_colored_rect(rect, dark_color);

    let mut top = true;
    for i in 0..n {
        let x = lerp(rect.left()..=rect.right(), i as f32 / (n as f32));
        let small_rect = if top {
            Rect::from_min_size(pos2(x, rect.top()), checker_size)
        } else {
            Rect::from_min_size(pos2(x, rect.center().y), checker_size)
        };
        mesh.add_colored_rect(small_rect, bright_color);
        top = !top;
    }
    painter.add(Shape::mesh(mesh));
}

/// Show a color with background checkers to demonstrate transparency (if any).
fn show_srgba_unmultiplied(ui: &mut Ui, srgba: [u8; 4], desired_size: Vec2) -> Response {
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::hover());
    if ui.is_rect_visible(rect) {
        show_srgba_unmultiplied_at(ui.painter(), srgba, rect);
    }
    response
}

/// Show a color with background checkers to demonstrate transparency (if any).
fn show_srgba_unmultiplied_at(painter: &Painter, [r, g, b, a]: [u8; 4], rect: Rect) {
    if a == 255 {
        painter.rect_filled(rect, 0.0, Color32::from_rgb(r, g, b));
    } else {
        background_checkers(painter, rect);
        let left = Rect::from_min_max(rect.left_top(), rect.center_bottom());
        let right = Rect::from_min_max(rect.center_top(), rect.right_bottom());
        painter.rect_filled(left, 0.0, Color32::from_rgba_unmultiplied(r, g, b, a));
        painter.rect_filled(right, 0.0, Color32::from_rgb(r, g, b));
    }
}

fn color_button(ui: &mut Ui, srgba: [u8; 4], open: bool) -> Response {
    let size = ui.spacing().interact_size;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::new(Role::ColorWell));

    if ui.is_rect_visible(rect) {
        let visuals = if open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };
        let rect = rect.expand(visuals.expansion);

        let stroke_width = 1.0;
        show_srgba_unmultiplied_at(ui.painter(), srgba, rect.shrink(stroke_width));

        let corner_radius = visuals.corner_radius.at_most(2); // Can't do more rounding because the background grid doesn't do any rounding
        ui.painter().rect_stroke(
            rect,
            corner_radius,
            (stroke_width, visuals.bg_fill), // Using fill for stroke is intentional, because default style has no border
            StrokeKind::Inside,
        );
    }

    response
}

fn color_slider_1d(ui: &mut Ui, value: &mut f32, color_at: impl Fn(f32) -> Color32) -> (Response, bool) {
    #![expect(clippy::identity_op)]

    let desired_size = vec2(ui.spacing().slider_width, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

    if let Some(mpos) = response.interact_pointer_pos() {
        *value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
    }

    let cur_pointer = response.interact_pointer_pos().is_some();
    let last_pointer = ui.ctx().data_mut(|d| {
        let v: &mut bool = d.get_temp_mut_or_default(response.id.with("slider_1d"));
        std::mem::replace(v, cur_pointer)
    });
    let released = last_pointer && (!cur_pointer || ui.input(|i| i.pointer.any_released()));

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);

        background_checkers(ui.painter(), rect); // for alpha:

        {
            // fill color:
            let mut mesh = Mesh::default();
            for i in 0..=N {
                let t = i as f32 / (N as f32);
                let color = color_at(t);
                let x = lerp(rect.left()..=rect.right(), t);
                mesh.colored_vertex(pos2(x, rect.top()), color);
                mesh.colored_vertex(pos2(x, rect.bottom()), color);
                if i < N {
                    mesh.add_triangle(2 * i + 0, 2 * i + 1, 2 * i + 2);
                    mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
                }
            }
            ui.painter().add(Shape::mesh(mesh));
        }

        ui.painter()
            .rect_stroke(rect, 0.0, visuals.bg_stroke, StrokeKind::Inside); // outline

        {
            // Show where the slider is at:
            let x = lerp(rect.left()..=rect.right(), *value);
            let r = rect.height() / 4.0;
            let picked_color = color_at(*value);
            ui.painter().add(Shape::convex_polygon(
                vec![
                    pos2(x, rect.center().y),   // tip
                    pos2(x + r, rect.bottom()), // right bottom
                    pos2(x - r, rect.bottom()), // left bottom
                ],
                picked_color,
                Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
            ));
        }
    }

    let released = response.lost_focus() || response.drag_stopped() || released;
    (response, released)
}

/// # Arguments
/// * `x_value` - X axis, either saturation or value (0.0-1.0).
/// * `y_value` - Y axis, either saturation or value (0.0-1.0).
/// * `color_at` - A function that dictates how the mix of saturation and value will be displayed in the 2d slider.
///
/// e.g.: `|x_value, y_value| HsvaSrgb { h: 1.0, s: x_value, v: y_value, a: 1.0 }.into()` displays the colors as follows:
/// * top-left: white `[s: 0.0, v: 1.0]`
/// * top-right: fully saturated color `[s: 1.0, v: 1.0]`
/// * bottom-right: black `[s: 0.0, v: 1.0].`
fn color_slider_2d(
    ui: &mut Ui,
    x_value: &mut f32,
    y_value: &mut f32,
    color_at: impl Fn(f32, f32) -> Color32,
) -> (Response, bool) {
    let desired_size = Vec2::splat(ui.spacing().slider_width);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

    if let Some(mpos) = response.interact_pointer_pos() {
        *x_value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
        *y_value = remap_clamp(mpos.y, rect.bottom()..=rect.top(), 0.0..=1.0);
    }

    let cur_pointer = response.interact_pointer_pos().is_some();
    let last_pointer = ui.ctx().data_mut(|d| {
        let v: &mut bool = d.get_temp_mut_or_default(response.id.with("slider_1d"));
        std::mem::replace(v, cur_pointer)
    });
    let released = last_pointer && (!cur_pointer || ui.input(|i| i.pointer.any_released()));

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let mut mesh = Mesh::default();

        for xi in 0..=N {
            for yi in 0..=N {
                let xt = xi as f32 / (N as f32);
                let yt = yi as f32 / (N as f32);
                let color = color_at(xt, yt);
                let x = lerp(rect.left()..=rect.right(), xt);
                let y = lerp(rect.bottom()..=rect.top(), yt);
                mesh.colored_vertex(pos2(x, y), color);

                if xi < N && yi < N {
                    let x_offset = 1;
                    let y_offset = N + 1;
                    let tl = yi * y_offset + xi;
                    mesh.add_triangle(tl, tl + x_offset, tl + y_offset);
                    mesh.add_triangle(tl + x_offset, tl + y_offset, tl + y_offset + x_offset);
                }
            }
        }
        ui.painter().add(Shape::mesh(mesh)); // fill

        ui.painter()
            .rect_stroke(rect, 0.0, visuals.bg_stroke, StrokeKind::Inside); // outline

        // Show where the slider is at:
        let x = lerp(rect.left()..=rect.right(), *x_value);
        let y = lerp(rect.bottom()..=rect.top(), *y_value);
        let picked_color = color_at(*x_value, *y_value);
        ui.painter().add(epaint::CircleShape {
            center: pos2(x, y),
            radius: rect.width() / 12.0,
            fill: picked_color,
            stroke: Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
        });
    }

    let released = response.lost_focus() || response.drag_stopped() || released;
    (response, released)
}

/// We use a negative alpha for additive colors within this file (a bit ironic).
///
/// We use alpha=0 to mean "transparent".
fn is_additive_alpha(a: f32) -> bool {
    a < 0.0
}

/// What options to show for alpha
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Alpha {
    /// Set alpha to 1.0, and show no option for it.
    Opaque,

    /// Only show normal blend options for alpha.
    OnlyBlend,

    /// Show both blend and additive options.
    BlendOrAdditive,
}

fn color_picker_hsvas_2d(ui: &mut Ui, hsvas: &mut HsvaSrgb, alpha: Alpha) -> (bool, bool) {
    let hsvas_start = *hsvas;
    let mut change_ended = false;

    let alpha_control = if is_additive_alpha(hsvas.a) {
        Alpha::Opaque // no alpha control for additive colors
    } else {
        alpha
    };

    let mut rgba_unmultiplied = hsvas.to_rgba_unmultiplied();
    // Only update if changed to avoid rounding issues.
    let (edit_changed, edit_change_ended) =
        rgba_edit_ui(ui, &mut rgba_unmultiplied, alpha_control);
    if edit_changed {
        if is_additive_alpha(hsvas.a) {
            let alpha = hsvas.a;

            *hsvas = HsvaSrgb::from_rgba_unmultiplied(rgba_unmultiplied);

            // Don't edit the alpha:
            hsvas.a = alpha;
        } else {
            // Normal blending.
            *hsvas = HsvaSrgb::from_rgba_unmultiplied(rgba_unmultiplied);
        }
    }
    change_ended |= edit_change_ended;

    let current_color_size = vec2(ui.spacing().slider_width, ui.spacing().interact_size.y);
    show_srgba_unmultiplied(ui, Color32::from(*hsvas).to_array(), current_color_size)
        .on_hover_text(f!("picker-selected"));

    if alpha == Alpha::BlendOrAdditive {
        let a = &mut hsvas.a;
        let mut additive = is_additive_alpha(*a);
        ui.horizontal(|ui| {
            ui.label("Blending:");
            change_ended |= ui.radio_value(&mut additive, false, "Normal").changed();
            change_ended |= ui.radio_value(&mut additive, true, "Additive").changed();

            if additive {
                *a = -a.abs();
            }

            if !additive {
                *a = a.abs();
            }
        });
    }

    let opaque = HsvaSrgb { a: 1.0, ..*hsvas };

    let HsvaSrgb { h, s, v, a: _ } = hsvas;

    if false {
        color_slider_1d(ui, s, |s| HsvaSrgb { s, ..opaque }.into()).0.on_hover_text("Saturation");
    }

    if false {
        color_slider_1d(ui, v, |v| HsvaSrgb { v, ..opaque }.into()).0.on_hover_text("Value");
    }

    let (_, slider_change_ended) = color_slider_2d(ui, s, v, |s, v| HsvaSrgb { s, v, ..opaque }.into());
    change_ended |= slider_change_ended;

    let (slider_response, slider_change_ended) = color_slider_1d(ui, h, |h| {
        HsvaSrgb {
            h,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        }
        .into()
    });
    slider_response.on_hover_text(f!("picker-hue"));
    change_ended |= slider_change_ended;

    let additive = is_additive_alpha(hsvas.a);

    if alpha == Alpha::Opaque {
        hsvas.a = 1.0;
    } else {
        let a = &mut hsvas.a;

        if alpha == Alpha::OnlyBlend {
            if is_additive_alpha(*a) {
                *a = 0.5; // was additive, but isn't allowed to be
            }
            let (slider_response, slider_change_ended) =
                color_slider_1d(ui, a, |a| HsvaSrgb { a, ..opaque }.into());
            slider_response.on_hover_text(f!("picker-alpha"));
            change_ended |= slider_change_ended;
        } else if !additive {
            let (slider_response, slider_change_ended) =
                color_slider_1d(ui, a, |a| HsvaSrgb { a, ..opaque }.into());
            slider_response.on_hover_text(f!("picker-alpha"));
            change_ended |= slider_change_ended;
        }
    }

    let changed = hsvas_start != *hsvas;
    (changed, change_ended)
}

/// Shows 4 `DragValue` widgets to be used to edit the RGBA f32 values.
/// Alpha's `DragValue` is hidden when `Alpha::Opaque`.
///
/// Returns `true` on change.
fn rgba_edit_ui(ui: &mut Ui, [r, g, b, a]: &mut [f32; 4], alpha: Alpha) -> (bool, bool) {
    fn drag_value(ui: &mut Ui, prefix: &str, tooltip: &str, value: &mut f32) -> (bool, bool) {
        let widget = DragValue::new(value)
            .speed(0.003)
            .prefix(prefix)
            .range(0.0..=1.0)
            .custom_formatter(|n, _| format!("{n:.03}"));
        let response = ui.add_sized([74.0, ui.spacing().interact_size.y], widget)
            .on_hover_text(tooltip);
        (response.changed(), response.drag_stopped() || response.lost_focus())
    }

    ui.horizontal(|ui| {
        let r_change = drag_value(ui, "R ", &f!("picker-red"), r);
        let g_change = drag_value(ui, "G ", &f!("picker-green"), g);
        let b_change = drag_value(ui, "B ", &f!("picker-blue"), b);
        let a_change = if alpha != Alpha::Opaque {
            drag_value(ui, "A ", &f!("picker-alpha"), a)
        } else {
            (false, false)
        };
        (r_change.0 || g_change.0 || b_change.0 || a_change.0,
         r_change.1 || g_change.1 || b_change.1 || a_change.1)
    }).inner
}

/// Shows a color picker where the user can change the given [`HsvaSrgb`] color.
///
/// Returns `true` on change.
pub fn color_edit_button_hsvas(ui: &mut Ui, hsvas: &mut HsvaSrgb, alpha: Alpha) -> (bool, bool) {
    let popup_id = ui.auto_id_with("popup");
    let open = Popup::is_id_open(ui.ctx(), popup_id);
    let mut button_response = color_button(ui, Color32::from(*hsvas).to_array(), open);
    if ui.style().explanation_tooltips {
        button_response = button_response.on_hover_text("Click to edit color");
    }

    const COLOR_SLIDER_WIDTH: f32 = 320.0;

    Popup::menu(&button_response)
        .id(popup_id)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.spacing_mut().slider_width = COLOR_SLIDER_WIDTH;
            color_picker_hsvas_2d(ui, hsvas, alpha)
        })
        .map_or((false, false), |r| r.inner)
}

/// Shows a button with the given color.
/// If the user clicks the button, a full color picker is shown.
pub fn color_edit_button_srgba(ui: &mut Ui, tint: &mut (f32, f32, f32, f32), alpha: Alpha) -> (bool, bool) {
    let rgba = [tint.0, tint.1, tint.2, tint.3];
    let mut hsvas = color_cache_get(ui.ctx(), rgba);
    let response = color_edit_button_hsvas(ui, &mut hsvas, alpha);
    let rgba = hsvas.to_rgba_unmultiplied();
    color_cache_set(ui.ctx(), rgba, hsvas);
    *tint = rgba.into();
    response
}

// [f32; 4] as a hashable cache key
#[derive(Clone, Copy, PartialEq)]
struct SrgbaKey([f32; 4]);

impl core::hash::Hash for SrgbaKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        for &c in &self.0 {
            let bits = if c == 0.0 { 0 } else { c.to_bits() };
            core::hash::Hash::hash(&bits, state);
        }
    }
}

// To ensure we keep hue slider when `srgba` is gray we store the full [`HsvaSrgb`] in a cache:
fn color_cache_get(ctx: &Context, srgba: [f32; 4]) -> HsvaSrgb {
    use_color_cache(ctx, |cc| cc.get(&SrgbaKey(srgba)).copied())
        .unwrap_or_else(|| HsvaSrgb::from_rgba_unmultiplied(srgba))
}

// To ensure we keep hue slider when `srgba` is gray we store the full [`HsvaSrgb`] in a cache:
fn color_cache_set(ctx: &Context, srgba: [f32; 4], hsvas: HsvaSrgb) {
    use_color_cache(ctx, |cc| cc.set(SrgbaKey(srgba), hsvas));
}

// To ensure we keep hue slider when `srgba` is gray we store the full [`HsvaSrgb`] in a cache:
fn use_color_cache<R>(ctx: &Context, f: impl FnOnce(&mut FixedCache<SrgbaKey, HsvaSrgb>) -> R) -> R {
    ctx.data_mut(|d| f(d.get_temp_mut_or_default(Id::NULL)))
}

// Copied from egui-0.36.2: egui/util/fixed_cache.rs
// Copyright (c) 2018-2021 Emil Ernerfeldt <emil.ernerfeldt@gmail.com>

use epaint::util::hash;

const FIXED_CACHE_SIZE: usize = 1024; // must be small for web/WASM build (for unknown reason)

/// Very stupid/simple key-value cache. TODO(emilk): improve
#[derive(Clone)]
pub(crate) struct FixedCache<K, V>([Option<(K, V)>; FIXED_CACHE_SIZE]);

impl<K, V> Default for FixedCache<K, V>
where
    K: Copy,
    V: Copy,
{
    fn default() -> Self {
        Self([None; FIXED_CACHE_SIZE])
    }
}

impl<K, V> core::fmt::Debug for FixedCache<K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Cache")
    }
}

impl<K, V> FixedCache<K, V>
where
    K: core::hash::Hash + PartialEq,
{
    pub fn get(&self, key: &K) -> Option<&V> {
        let bucket = (hash(key) % (FIXED_CACHE_SIZE as u64)) as usize;
        match &self.0[bucket] {
            Some((k, v)) if k == key => Some(v),
            _ => None,
        }
    }

    pub fn set(&mut self, key: K, value: V) {
        let bucket = (hash(&key) % (FIXED_CACHE_SIZE as u64)) as usize;
        self.0[bucket] = Some((key, value));
    }
}
