//! `neb-wallet-gui`: the NEB wallet in a window. One home screen with the balance and three actions, then one screen
//! per action, in the look of the NEB terminal programs. The key file is the one of `neb-wallet`, encrypted with a
//! password; keys, the scan of the chain, coin selection and signing are those of the `neb-wallet` library.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod qr;
mod remote;
mod screens;
mod ui;

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Context, FontFamily, Frame, Margin, Stroke, TextureHandle, TextureOptions, vec2};
use neb_wallet::Wallet;

use backend::Balance;
use ui::{CARD, GREEN, GROUND};

/// The wallet locks itself after this time without a key press or a click.
const LOCK_AFTER: Duration = Duration::from_secs(5 * 60);

fn main() -> eframe::Result {
    let (rgba, width, height) = decode(include_bytes!("../assets/logo-mark.png"));
    let viewport = egui::ViewportBuilder::default()
        .with_title("NEB Wallet")
        .with_inner_size([440.0, 780.0])
        .with_min_inner_size([380.0, 620.0])
        .with_icon(egui::IconData { rgba, width, height });
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("NEB Wallet", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

fn decode(png: &[u8]) -> (Vec<u8>, u32, u32) {
    let image = image::load_from_memory(png).expect("a logo image built into the program").to_rgba8();
    let (width, height) = image.dimensions();
    (image.into_raw(), width, height)
}

fn texture(ctx: &Context, name: &str, png: &[u8]) -> TextureHandle {
    let (rgba, width, height) = decode(png);
    let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}

/// Why a new password is asked.
#[derive(Clone)]
enum Purpose {
    Create,
    /// With the 64-hex secret to keep.
    Import(String),
}

enum Screen {
    Welcome,
    ImportKey,
    NewPassword(Purpose),
    Locked,
    Home,
    Accounts,
    AddKey,
    Receive,
    Send,
    Review,
    /// With the transaction id.
    Sent(String),
    Settings,
    KeyPassword,
    /// With the secret, and whether the wallet was just created.
    Key(String, bool),
    /// Placeholder while a screen is drawn.
    Drawing,
}

/// What a background task brings back.
enum Msg {
    /// A wallet that was opened or created; with its secret when it is new, to back it up.
    Opened(Result<(Wallet, Option<String>), String>),
    /// With the account that was asked about.
    Balance(usize, Result<Balance, String>),
    Sent(Result<String, String>),
    Key(Result<String, String>),
}

struct App {
    screen: Screen,
    /// The key file, with every key.
    file: Option<Wallet>,
    /// The account on the screen: the position of its key in the file.
    account: usize,
    /// That account alone, as a wallet of one key: what the screens show and what a payment spends.
    wallet: Option<Arc<Wallet>>,
    node: String,
    balance: Option<Balance>,
    node_error: String,
    refreshing: bool,
    /// A task that the screen waits for.
    busy: bool,
    /// The amounts are hidden.
    hidden: bool,
    error: String,
    /// Input fields. The password fields are cleared as soon as they are used.
    password: String,
    again: String,
    secret: String,
    to: String,
    amount: String,
    node_edit: String,
    key_shown: bool,
    copied: Option<Instant>,
    last_active: Instant,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    mark: TextureHandle,
    logo: TextureHandle,
    /// The code of the address, drawn once.
    code: Option<qr::Modules>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let ctx = &cc.egui_ctx;
        let mut style = (*ctx.style()).clone();
        for font in style.text_styles.values_mut() {
            font.family = FontFamily::Monospace;
        }
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = GROUND;
        style.visuals.extreme_bg_color = CARD;
        style.visuals.selection.bg_fill = Color32::from_rgb(25, 62, 40);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, GREEN);
        style.spacing.item_spacing = vec2(8.0, 10.0);
        style.spacing.button_padding = vec2(14.0, 10.0);
        ctx.set_style(style);

        let (tx, rx) = channel();
        let node = backend::load_node();
        let mut app = App {
            screen: if backend::wallet_path().exists() { Screen::Locked } else { Screen::Welcome },
            file: None,
            account: 0,
            wallet: None,
            node_edit: node.clone(),
            node,
            balance: None,
            node_error: String::new(),
            refreshing: false,
            busy: false,
            hidden: false,
            error: String::new(),
            password: String::new(),
            again: String::new(),
            secret: String::new(),
            to: String::new(),
            amount: String::new(),
            key_shown: false,
            copied: None,
            last_active: Instant::now(),
            tx,
            rx,
            mark: texture(ctx, "mark", include_bytes!("../assets/logo-mark.png")),
            logo: texture(ctx, "logo", include_bytes!("../assets/logo.png")),
            code: None,
        };
        app.developer_start(ctx);
        app
    }

    /// Debug builds only: opens the wallet with `NEB_WALLET_PASSWORD` and goes to the screen named by
    /// `NEB_WALLET_SCREEN`, to look at a screen without typing. A release build has none of this.
    #[cfg(debug_assertions)]
    fn developer_start(&mut self, ctx: &Context) {
        let Ok(password) = std::env::var("NEB_WALLET_PASSWORD") else { return };
        let Ok(wallet) = backend::unlock(&backend::wallet_path(), &password) else { return };
        self.open(ctx, wallet);
        match std::env::var("NEB_WALLET_SCREEN").as_deref() {
            Ok("receive") => self.screen = Screen::Receive,
            Ok("send") => self.screen = Screen::Send,
            Ok("review") => {
                self.to = backend::address(self.wallet.as_ref().expect("an open wallet"));
                self.amount = "1.5".into();
                self.screen = Screen::Review;
            }
            Ok("settings") => self.screen = Screen::Settings,
            Ok("key") => self.screen = Screen::Key("0".repeat(64), true),
            _ => {}
        }
    }

    #[cfg(not(debug_assertions))]
    fn developer_start(&mut self, _ctx: &Context) {}

    /// Runs `job` on another thread: deriving a key from a password and reading the chain both take time.
    fn spawn(&self, ctx: &Context, job: impl FnOnce() -> Msg + Send + 'static) {
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            // The window may be closed before the task ends: nobody is left to tell.
            let _ = tx.send(job());
            ctx.request_repaint();
        });
    }

    fn open(&mut self, ctx: &Context, wallet: Wallet) {
        self.file = Some(wallet);
        self.screen = Screen::Home;
        self.last_active = Instant::now();
        self.select(ctx, 0);
    }

    /// Puts one account of the key file on the screen: its address, its code, its balance.
    fn select(&mut self, ctx: &Context, account: usize) {
        let Some(view) = self.file.as_ref().and_then(|file| backend::account(file, account)) else { return };
        self.account = account;
        self.code = qr::encode(&backend::address(&view));
        self.wallet = Some(Arc::new(view));
        self.balance = None;
        self.node_error.clear();
        self.refresh(ctx);
    }

    fn lock(&mut self) {
        self.file = None;
        self.wallet = None;
        self.balance = None;
        self.code = None;
        self.node_error.clear();
        self.error.clear();
        self.password.clear();
        self.again.clear();
        self.to.clear();
        self.amount.clear();
        self.screen = Screen::Locked;
    }

    /// Asks the node for the balance. The whole chain is read each time: slow on a long chain.
    fn refresh(&mut self, ctx: &Context) {
        let Some(wallet) = self.wallet.clone() else { return };
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let (node, account) = (self.node.clone(), self.account);
        self.spawn(ctx, move || Msg::Balance(account, backend::balance(&wallet, &node)));
    }

    fn receive(&mut self, ctx: &Context, msg: Msg) {
        match msg {
            Msg::Opened(Ok((wallet, secret))) => {
                self.busy = false;
                self.error.clear();
                self.open(ctx, wallet);
                if let Some(secret) = secret {
                    self.key_shown = false;
                    self.screen = Screen::Key(secret, true);
                }
            }
            Msg::Opened(Err(e)) => {
                self.busy = false;
                self.error = if matches!(self.screen, Screen::Locked) { "Wrong password.".into() } else { e };
            }
            Msg::Balance(account, result) => {
                self.refreshing = false;
                // The answer is about another account than the one on the screen: it is asked again.
                if account != self.account {
                    self.refresh(ctx);
                    return;
                }
                match result {
                    Ok(balance) => {
                        self.balance = Some(balance);
                        self.node_error.clear();
                    }
                    Err(e) => self.node_error = e,
                }
            }
            Msg::Sent(result) => {
                self.busy = false;
                match result {
                    Ok(txid) => {
                        self.to.clear();
                        self.amount.clear();
                        self.error.clear();
                        self.screen = Screen::Sent(txid);
                        self.refresh(ctx);
                    }
                    Err(e) => self.error = e,
                }
            }
            Msg::Key(result) => {
                self.busy = false;
                match result {
                    Ok(secret) => {
                        self.error.clear();
                        self.key_shown = false;
                        self.screen = Screen::Key(secret, false);
                    }
                    Err(_) => self.error = "Wrong password.".into(),
                }
            }
        }
    }

    fn go(&mut self, screen: Screen) {
        self.error.clear();
        self.screen = screen;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            self.receive(ctx, msg);
        }
        // The wallet locks itself when nobody uses it.
        if ctx.input(|i| !i.events.is_empty() || i.pointer.any_down()) {
            self.last_active = Instant::now();
        }
        if self.wallet.is_some() {
            if self.last_active.elapsed() > LOCK_AFTER {
                self.lock();
            }
            ctx.request_repaint_after(Duration::from_secs(1));
        }

        let frame = Frame::none().fill(GROUND).inner_margin(Margin::same(20.0));
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            // The screen is taken out while it is drawn, so that it can hold data and still change the state.
            let screen = std::mem::replace(&mut self.screen, Screen::Drawing);
            match &screen {
                Screen::Welcome => self.welcome(ui),
                Screen::ImportKey => self.import_key(ui),
                Screen::NewPassword(purpose) => self.new_password(ui, purpose.clone()),
                Screen::Locked => self.locked(ui),
                Screen::Home => self.home(ui),
                Screen::Accounts => self.accounts(ui),
                Screen::AddKey => self.add_key(ui),
                Screen::Receive => self.receive_screen(ui),
                Screen::Send => self.send_screen(ui),
                Screen::Review => self.review(ui),
                Screen::Sent(txid) => self.sent(ui, txid),
                Screen::Settings => self.settings(ui),
                Screen::KeyPassword => self.key_password(ui),
                Screen::Key(secret, new) => self.key(ui, secret, *new),
                Screen::Drawing => {}
            }
            // A screen that asked for another one has replaced the placeholder; otherwise the same screen stays.
            if matches!(self.screen, Screen::Drawing) {
                self.screen = screen;
            }
        });
    }
}
