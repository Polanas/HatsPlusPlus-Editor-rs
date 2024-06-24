use eframe::egui::{Key, KeyboardShortcut, Modifiers};
//BUG shortcuts work only with english layout
pub const HOME: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::H);
pub const PAUSE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Space);
pub const DECREASE_FRAME: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::ArrowLeft);
pub const INCREASE_FRAME: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::ArrowRight);
pub const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::O);
pub const NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::N);
pub const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::S);
pub const SAVE_AS: KeyboardShortcut = KeyboardShortcut::new(
    Modifiers {
        ctrl: false,
        shift: true,
        alt: false,
        mac_cmd: false,
        command: false,
    },
    Key::S,
);
