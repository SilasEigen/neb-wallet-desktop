//! The screens of the wallet, one function each.

use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Color32, Frame, Layout, Margin, Rounding, Sense, Stroke, Ui, Vec2, pos2, vec2};

use neb_wallet::WalletKey;

use crate::ui::{
    CARD, EDGE, GREEN, GREY, GROUND, WHITE, account_row, caption, card, entered, field, round, text, tile, title, wide,
    wide_in_card, wrapped,
};
use crate::{App, Msg, Purpose, Screen, backend, qr};

impl App {
    pub(crate) fn brand(&self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.image((self.logo.id(), vec2(190.0, 193.0)));
            ui.add_space(18.0);
        });
    }

    pub(crate) fn error_line(&self, ui: &mut Ui) {
        if !self.error.is_empty() {
            ui.label(text(&self.error, 13.0, Color32::from_rgb(255, 135, 135)));
        }
        if self.busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(text("working...", 13.0, GREY));
            });
        }
    }

    pub(crate) fn welcome(&mut self, ui: &mut Ui) {
        self.brand(ui);
        ui.vertical_centered(|ui| {
            ui.label(text("Your NEB wallet", 22.0, WHITE).strong());
            ui.label(text("Your keys, kept on this computer and encrypted\nwith a password you choose.", 14.0, GREY));
        });
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, "Import a key", false, true) {
                self.secret.clear();
                self.go(Screen::ImportKey);
            }
            if wide(ui, "Create a wallet", true, true) {
                self.go(Screen::NewPassword(Purpose::Create));
            }
        });
    }

    pub(crate) fn import_key(&mut self, ui: &mut Ui) {
        if title(ui, "Import a key") {
            self.go(Screen::Welcome);
            return;
        }
        caption(ui, "SECRET KEY (64 HEX CHARACTERS)");
        field(ui, &mut self.secret, "", true);
        let valid = self.secret.trim().len() == 64 && self.secret.trim().chars().all(|c| c.is_ascii_hexdigit());
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, if valid { "Continue" } else { "Enter Key" }, true, valid) {
                let secret = std::mem::take(&mut self.secret).trim().to_string();
                self.go(Screen::NewPassword(Purpose::Import(secret)));
            }
        });
    }

    pub(crate) fn new_password(&mut self, ui: &mut Ui, purpose: Purpose) {
        if title(ui, "Password") {
            self.password.clear();
            self.again.clear();
            self.go(Screen::Welcome);
            return;
        }
        ui.label(text(
            "It encrypts the key on this computer and opens the wallet. It cannot be recovered: keep it with the \
             backup of the key.",
            13.0,
            GREY,
        ));
        ui.add_space(8.0);
        caption(ui, "PASSWORD (8 CHARACTERS OR MORE)");
        field(ui, &mut self.password, "", true);
        caption(ui, "AGAIN");
        field(ui, &mut self.again, "", true);
        let (long, same) = (self.password.chars().count() >= 8, self.password == self.again);
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            let label = if !long {
                "Enter Password"
            } else if !same {
                "Repeat Password"
            } else {
                "Continue"
            };
            if wide(ui, label, true, long && same && !self.busy) {
                let password = std::mem::take(&mut self.password);
                self.again.clear();
                self.busy = true;
                let path = backend::wallet_path();
                self.spawn(ui.ctx(), move || {
                    Msg::Opened(match purpose {
                        Purpose::Create => backend::create(&path, &password).map(|w| {
                            let secret = w.keys.first().map(|k| k.secret_hex());
                            (w, secret)
                        }),
                        Purpose::Import(secret) => backend::import(&path, &secret, &password).map(|w| (w, None)),
                    })
                });
            }
        });
    }

    pub(crate) fn locked(&mut self, ui: &mut Ui) {
        self.brand(ui);
        caption(ui, "PASSWORD");
        let response = field(ui, &mut self.password, "", true);
        let enter = entered(ui, &response);
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            let ready = !self.password.is_empty() && !self.busy;
            if (wide(ui, "Unlock", true, ready) || enter) && ready {
                let password = std::mem::take(&mut self.password);
                self.busy = true;
                let path = backend::wallet_path();
                self.spawn(ui.ctx(), move || Msg::Opened(backend::unlock(&path, &password).map(|w| (w, None))));
            }
        });
    }

    pub(crate) fn home(&mut self, ui: &mut Ui) {
        // Top bar: the brand on the left; hide the amounts, and the settings, on the right.
        ui.horizontal(|ui| {
            // The brand names the account on the screen and opens the list of accounts.
            let name = format!("NEB  Account {}", self.account + 1);
            let brand = Frame::none()
                .fill(CARD)
                .stroke(Stroke::new(1.0_f32, EDGE))
                .rounding(Rounding::same(22.0))
                .inner_margin(Margin::symmetric(12.0, 9.0))
                .show(ui, |ui| {
                    ui.image((self.mark.id(), vec2(26.0, 23.0)));
                    ui.add(egui::Label::new(text(name, 14.0, WHITE).strong()).selectable(false));
                });
            if brand.response.interact(Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.go(Screen::Accounts);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if round(ui, "\u{2699}", WHITE) {
                    self.node_edit = self.node.clone();
                    self.go(Screen::Settings);
                }
                if round(ui, "\u{1f441}", if self.hidden { EDGE } else { WHITE }) {
                    self.hidden = !self.hidden;
                }
            });
        });

        // The balance, alone in the middle.
        let hidden = self.hidden;
        let hide = move |s: &str| if hidden { "****".to_string() } else { s.to_string() };
        ui.vertical_centered(|ui| {
            ui.add_space(34.0);
            let amount = self.balance.as_ref().map_or("-".to_string(), |b| hide(&b.spendable));
            ui.label(text(amount, 50.0, WHITE).strong());
            ui.label(text("NEB", 16.0, GREEN).strong());
            let (state, ink) = if self.refreshing {
                ("asking the node...".to_string(), GREY)
            } else if !self.node_error.is_empty() {
                ("\u{25cb} node offline".to_string(), GREY)
            } else if let Some(b) = &self.balance {
                (format!("\u{25cf} block {}{}", b.height, if b.public { " \u{b7} public node" } else { "" }), GREEN)
            } else {
                (String::new(), GREY)
            };
            ui.label(text(state, 13.0, ink));
            ui.add_space(24.0);
        });

        // Three actions.
        ui.columns(3, |columns| {
            if tile(&mut columns[0], "\u{2193}", "Receive") {
                self.go(Screen::Receive);
            }
            if tile(&mut columns[1], "\u{2191}", "Send") {
                self.go(Screen::Send);
            }
            if tile(&mut columns[2], "\u{27f3}", "Refresh") {
                self.refresh(columns[2].ctx());
            }
        });

        // What the node says of the coins that are not spendable yet, or why there is no figure.
        ui.add_space(24.0);
        ui.label(text("Coins", 20.0, WHITE).strong());
        card(ui, |ui| {
            if !self.node_error.is_empty() {
                ui.label(text("The node did not answer.", 14.0, WHITE));
                ui.label(text(&self.node_error, 12.0, GREY));
                ui.label(text(format!("Node: {}", self.node), 12.0, GREY));
            } else if let Some(b) = self.balance.as_ref().filter(|b| !b.maturing.is_empty()) {
                ui.label(text(format!("Spendable   {} NEB", hide(&b.spendable)), 14.0, WHITE));
                ui.label(text(format!("Maturing  + {} NEB", hide(&b.maturing)), 14.0, GREEN));
                ui.label(text("A mined block can be spent 100 blocks after it was found.", 12.0, GREY));
            } else {
                let first = if self.balance.is_some() { "Nothing is maturing." } else { "No figure yet." };
                ui.label(text(first, 14.0, WHITE));
                ui.label(text("Coins you receive or mine appear here.", 12.0, GREY));
            }
        });
    }

    pub(crate) fn receive_screen(&mut self, ui: &mut Ui) {
        if title(ui, "Receive") {
            self.go(Screen::Home);
            return;
        }
        let address = self.wallet.as_ref().map(|w| backend::address(w)).unwrap_or_default();
        ui.vertical_centered(|ui| {
            if let Some(code) = &self.code {
                self.draw_code(ui, code, 250.0);
            }
            ui.add_space(14.0);
            caption(ui, "YOUR ADDRESS");
            ui.label(text(wrapped(&address), 17.0, WHITE));
            ui.add_space(8.0);
            ui.label(text("Give it to the sender,\nto your miner or to your pool.", 12.0, GREY));
        });
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            let fresh = self.copied.is_some_and(|t| t.elapsed() < Duration::from_secs(2));
            if wide(ui, if fresh { "Copied" } else { "Copy Address" }, true, true) {
                ui.ctx().copy_text(address.clone());
                self.copied = Some(Instant::now());
            }
        });
    }

    /// The address as a QR code, dark on white so that every scanner reads it, with the NEB mark in the middle.
    pub(crate) fn draw_code(&self, ui: &mut Ui, code: &qr::Modules, side: f32) {
        let (response, painter) = ui.allocate_painter(Vec2::splat(side), Sense::hover());
        let rect = response.rect;
        painter.rect_filled(rect, Rounding::same(18.0), Color32::WHITE);
        let quiet = side * 0.07;
        let cell = (side - 2.0 * quiet) / qr::SIZE as f32;
        for (y, row) in code.iter().enumerate() {
            for (x, dark) in row.iter().enumerate() {
                if *dark {
                    let min = rect.min + vec2(quiet + x as f32 * cell, quiet + y as f32 * cell);
                    // A little overlap hides the seams between modules.
                    painter.rect_filled(egui::Rect::from_min_size(min, Vec2::splat(cell + 0.5)), 0.0, Color32::BLACK);
                }
            }
        }
        // The mark hides 9 of the 41 modules across; the error correction of the code restores them.
        let half = 4.5 * cell;
        let middle = egui::Rect::from_center_size(rect.center(), Vec2::splat(2.0 * half));
        painter.rect_filled(middle, Rounding::same(cell), GROUND);
        let width = half * 1.5;
        let size = vec2(width, width * self.mark.size()[1] as f32 / self.mark.size()[0] as f32);
        let uv = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(self.mark.id(), egui::Rect::from_center_size(rect.center(), size), uv, Color32::WHITE);
    }

    pub(crate) fn send_screen(&mut self, ui: &mut Ui) {
        if title(ui, "Send") {
            self.go(Screen::Home);
            return;
        }
        caption(ui, "SEND TO");
        field(ui, &mut self.to, "Enter NEB address", false);
        backend::tidy(&mut self.to);
        ui.add_space(6.0);
        caption(ui, "AMOUNT");
        field(ui, &mut self.amount, "0.0", false);
        let spendable = self.balance.as_ref().map_or("-".to_string(), |b| b.spendable.clone());
        ui.label(text(format!("Spendable {spendable} NEB \u{b7} fee {} NEB", backend::fee()), 12.0, GREY));
        // The button says what is missing, and turns green once the payment can be reviewed.
        let (has_address, has_amount) = (backend::is_address(&self.to), backend::is_amount(&self.amount));
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            let label = if !has_address {
                "Enter Address"
            } else if !has_amount {
                "Enter Amount"
            } else {
                "Review"
            };
            if wide(ui, label, true, has_address && has_amount) {
                self.go(Screen::Review);
            }
        });
    }

    pub(crate) fn review(&mut self, ui: &mut Ui) {
        if title(ui, "Review") {
            self.go(Screen::Send);
            return;
        }
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.label(text(self.amount.trim(), 44.0, WHITE).strong());
            ui.label(text("NEB", 16.0, GREEN).strong());
            ui.add_space(14.0);
        });
        card(ui, |ui| {
            caption(ui, "TO");
            ui.label(text(wrapped(self.to.trim()), 15.0, WHITE));
            ui.add_space(6.0);
            caption(ui, "FEE");
            ui.label(text(format!("{} NEB", backend::fee()), 15.0, WHITE));
        });
        ui.label(text("A payment cannot be undone.", 12.0, GREY));
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, "Cancel", false, !self.busy) {
                self.go(Screen::Send);
            }
            if wide(ui, "Send", true, !self.busy) {
                let Some(wallet) = self.wallet.clone() else { return };
                self.busy = true;
                self.error.clear();
                let (node, to, amount) = (self.node.clone(), self.to.clone(), self.amount.clone());
                self.spawn(ui.ctx(), move || Msg::Sent(backend::send(&wallet, &node, &to, &amount)));
            }
        });
    }

    pub(crate) fn sent(&mut self, ui: &mut Ui, txid: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            ui.label(text("\u{2713}", 60.0, GREEN));
            ui.label(text("Sent", 24.0, WHITE).strong());
            ui.add_space(14.0);
            caption(ui, "TRANSACTION");
            ui.label(text(wrapped(txid), 13.0, WHITE));
        });
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, "Done", true, true) {
                self.go(Screen::Home);
            }
        });
    }

    pub(crate) fn settings(&mut self, ui: &mut Ui) {
        if title(ui, "Settings") {
            self.go(Screen::Home);
            return;
        }
        card(ui, |ui| {
            caption(ui, "NODE");
            ui.label(text("The wallet asks this node for the balance and gives it the payments.", 13.0, GREY));
            field(ui, &mut self.node_edit, backend::DEFAULT_NODE, false);
            if wide_in_card(ui, "Save", self.node_edit.trim() != self.node) {
                match backend::save_node(&self.node_edit) {
                    Ok(()) => {
                        self.node = self.node_edit.trim().to_string();
                        self.balance = None;
                        self.refresh(ui.ctx());
                        self.go(Screen::Home);
                    }
                    Err(e) => self.error = e,
                }
            }
            ui.label(text("The default is a neb-node running on this computer. When there is none,", 12.0, GREY));
            ui.label(text("the wallet asks the public node of the project.", 12.0, GREY));
        });
        ui.add_space(4.0);
        card(ui, |ui| {
            caption(ui, "KEY");
            let path = backend::shown_wallet_path();
            ui.label(text(format!("The key file is encrypted with your password:\n{path}"), 13.0, GREY));
            if wide_in_card(ui, "Show secret key", true) {
                self.password.clear();
                self.go(Screen::KeyPassword);
            }
        });
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, "Lock", false, true) {
                self.lock();
            }
        });
    }

    /// The accounts of the key file: one key each. A click shows an account; the buttons add one.
    pub(crate) fn accounts(&mut self, ui: &mut Ui) {
        if title(ui, "Accounts") {
            self.go(Screen::Home);
            return;
        }
        let addresses: Vec<String> =
            self.file.as_ref().map(|file| file.keys.iter().map(|k| k.address.clone()).collect()).unwrap_or_default();
        let mut chosen = None;
        egui::ScrollArea::vertical().max_height(ui.available_height() - 140.0).show(ui, |ui| {
            for (index, address) in addresses.iter().enumerate() {
                if account_row(ui, &format!("Account {}", index + 1), address, index == self.account) {
                    chosen = Some(index);
                }
            }
        });
        if let Some(index) = chosen {
            self.select(ui.ctx(), index);
            self.go(Screen::Home);
            return;
        }
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, "Import a key", false, true) {
                self.secret.clear();
                self.go(Screen::AddKey);
            }
            if wide(ui, "Create a key", true, true) {
                let key = WalletKey::generate();
                let secret = key.secret_hex();
                if self.add_account(ui.ctx(), key) {
                    self.key_shown = false;
                    self.screen = Screen::Key(secret, true);
                }
            }
        });
    }

    /// Imports one more key into the key file.
    pub(crate) fn add_key(&mut self, ui: &mut Ui) {
        if title(ui, "Import a key") {
            self.secret.clear();
            self.go(Screen::Accounts);
            return;
        }
        caption(ui, "SECRET KEY (64 HEX CHARACTERS)");
        field(ui, &mut self.secret, "", true);
        let valid = self.secret.trim().len() == 64 && self.secret.trim().chars().all(|c| c.is_ascii_hexdigit());
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, if valid { "Add" } else { "Enter Key" }, true, valid) {
                let secret = std::mem::take(&mut self.secret);
                match WalletKey::from_text(secret.trim()) {
                    Ok(key) => {
                        if self.add_account(ui.ctx(), key) {
                            self.go(Screen::Home);
                        }
                    }
                    Err(e) => self.error = e,
                }
            }
        });
    }

    /// Adds a key to the key file and shows its account. True when the key is in the file.
    // ponytail: the file is encrypted again on the thread of the window, about a second; a task if it is felt.
    fn add_account(&mut self, ctx: &egui::Context, key: WalletKey) -> bool {
        let Some(file) = self.file.as_mut() else { return false };
        match backend::add_key(file, key) {
            Ok(index) => {
                self.error.clear();
                self.select(ctx, index);
                true
            }
            Err(e) => {
                self.error = e;
                false
            }
        }
    }

    pub(crate) fn key_password(&mut self, ui: &mut Ui) {
        if title(ui, "Secret key") {
            self.password.clear();
            self.go(Screen::Settings);
            return;
        }
        caption(ui, "PASSWORD OF THE WALLET");
        let response = field(ui, &mut self.password, "", true);
        let enter = entered(ui, &response);
        self.error_line(ui);
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            let ready = !self.password.is_empty() && !self.busy;
            if (wide(ui, "Show", true, ready) || enter) && ready {
                let password = std::mem::take(&mut self.password);
                self.busy = true;
                let (path, account) = (backend::wallet_path(), self.account);
                self.spawn(ui.ctx(), move || {
                    Msg::Key(
                        backend::unlock(&path, &password)
                            .map(|w| w.keys.get(account).map(|k| k.secret_hex()).unwrap_or_default()),
                    )
                });
            }
        });
    }

    /// The secret key behind asterisks; the eye reveals it.
    pub(crate) fn key(&mut self, ui: &mut Ui, secret: &str, new: bool) {
        ui.add_space(6.0);
        ui.vertical_centered(|ui| ui.label(text("Secret key", 20.0, WHITE).strong()));
        ui.add_space(10.0);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                let shown: String = if self.key_shown { secret.to_string() } else { "*".repeat(secret.len()) };
                let lines: Vec<String> =
                    shown.as_bytes().chunks(16).map(|c| String::from_utf8_lossy(c).into_owned()).collect();
                ui.label(text(lines.join("\n"), 16.0, GREEN));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if round(ui, "\u{1f441}", if self.key_shown { WHITE } else { EDGE }) {
                        self.key_shown = !self.key_shown;
                    }
                });
            });
        });
        ui.label(text(
            "Write this key down and keep it off this computer. Anyone who has it can spend the coins.\n\nIf the key \
             file or its password is lost, the coins can only be recovered with this key: Import a key in this \
             program, or neb-wallet import KEY.",
            13.0,
            GREY,
        ));
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if wide(ui, if new { "I wrote it down" } else { "Done" }, true, true) {
                self.key_shown = false;
                self.go(if new { Screen::Home } else { Screen::Settings });
            }
            if wide(ui, "Copy", false, true) {
                ui.ctx().copy_text(secret.to_string());
            }
        });
    }
}
