//! Pieces of the look: a dark ground, one fixed-width font, green for what matters, rounded frames.

use eframe::egui::{
    self, Button, Color32, Direction, Frame, Layout, Margin, RichText, Rounding, Sense, Stroke, TextEdit, Ui, vec2,
};

pub const GROUND: Color32 = Color32::from_rgb(18, 18, 18);
pub const CARD: Color32 = Color32::from_rgb(28, 28, 28);
pub const EDGE: Color32 = Color32::from_rgb(68, 68, 68);
/// The green of the logo (`assets/logo.png`).
pub const GREEN: Color32 = Color32::from_rgb(84, 208, 135);
pub const WHITE: Color32 = Color32::from_rgb(238, 238, 238);
pub const GREY: Color32 = Color32::from_rgb(128, 128, 128);

pub fn text(s: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(s).size(size).color(color)
}

pub fn wide(ui: &mut Ui, label: &str, main: bool, enabled: bool) -> bool {
    let (fill, ink) = match (main, enabled) {
        (true, true) => (GREEN, GROUND),
        (true, false) => (CARD, GREY),
        (false, _) => (CARD, WHITE),
    };
    let button = Button::new(text(label, 16.0, ink).strong())
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, if main && enabled { GREEN } else { EDGE }))
        .rounding(Rounding::same(16.0))
        .min_size(vec2(ui.available_width(), 52.0));
    ui.add_enabled(enabled, button).clicked()
}

pub fn round(ui: &mut Ui, sign: &str, ink: Color32) -> bool {
    let button = Button::new(text(sign, 16.0, ink))
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, EDGE))
        .rounding(Rounding::same(22.0))
        .min_size(vec2(44.0, 44.0));
    ui.add(button).clicked()
}

pub fn card<R>(ui: &mut Ui, inside: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, EDGE))
        .rounding(Rounding::same(16.0))
        .inner_margin(Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            inside(ui)
        })
        .inner
}

/// A wide button inside a card. The card lays its content out from the left, so the button gets a justified
/// layout of its own to keep its label centred.
pub fn wide_in_card(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    ui.vertical_centered_justified(|ui| wide(ui, label, false, enabled)).inner
}

pub fn caption(ui: &mut Ui, s: &str) {
    ui.label(text(s, 12.0, GREY));
}

pub fn field(ui: &mut Ui, value: &mut String, hint: &str, password: bool) -> egui::Response {
    ui.add(
        TextEdit::singleline(value)
            .hint_text(text(hint, 15.0, EDGE))
            .password(password)
            .font(egui::FontId::monospace(15.0))
            .margin(Margin::symmetric(14.0, 14.0))
            .desired_width(f32::INFINITY),
    )
}

pub fn entered(ui: &Ui, response: &egui::Response) -> bool {
    response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

/// The address on three lines, as it is easier to compare.
pub fn wrapped(address: &str) -> String {
    address.as_bytes().chunks(21).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join("\n")
}

/// The top bar of an inner screen: the way back, and the title in the middle. True when back was clicked.
pub fn title(ui: &mut Ui, name: &str) -> bool {
    let mut back = false;
    ui.horizontal(|ui| {
        back = round(ui, "<", WHITE);
        // As much room is kept on the right as the button takes on the left, so that the title is centred.
        let width = ui.available_width() - 52.0;
        ui.allocate_ui_with_layout(vec2(width, 44.0), Layout::centered_and_justified(Direction::LeftToRight), |ui| {
            ui.label(text(name, 20.0, WHITE).strong());
        });
    });
    ui.add_space(14.0);
    back
}

/// A row of the list of accounts: its name on the left, its address in short on the right, a green edge when it is
/// the one shown. True when it was clicked.
pub fn account_row(ui: &mut Ui, name: &str, address: &str, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::click());
    let fill = if response.hovered() { Color32::from_rgb(36, 36, 36) } else { CARD };
    ui.painter().rect(rect, Rounding::same(16.0), fill, Stroke::new(1.0_f32, if active { GREEN } else { EDGE }));
    let font = |size| egui::FontId::monospace(size);
    // An address is ASCII: it is cut at byte positions.
    let short = if address.len() > 20 {
        format!("{}...{}", &address[..12], &address[address.len() - 6..])
    } else {
        address.to_string()
    };
    let ink = if active { GREEN } else { WHITE };
    ui.painter().text(rect.left_center() + vec2(16.0, 0.0), egui::Align2::LEFT_CENTER, name, font(15.0), ink);
    ui.painter().text(rect.right_center() - vec2(16.0, 0.0), egui::Align2::RIGHT_CENTER, short, font(13.0), GREY);
    response.clicked()
}

/// A tile of the home screen: a sign in green over a word, both centred. True when it was clicked.
pub fn tile(ui: &mut Ui, sign: &str, word: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 92.0), Sense::click());
    let fill = if response.hovered() { Color32::from_rgb(36, 36, 36) } else { CARD };
    ui.painter().rect(rect, Rounding::same(16.0), fill, Stroke::new(1.0_f32, EDGE));
    let font = |size| egui::FontId::monospace(size);
    ui.painter().text(rect.center() - vec2(0.0, 14.0), egui::Align2::CENTER_CENTER, sign, font(26.0), GREEN);
    ui.painter().text(rect.center() + vec2(0.0, 20.0), egui::Align2::CENTER_CENTER, word, font(14.0), WHITE);
    response.clicked()
}
