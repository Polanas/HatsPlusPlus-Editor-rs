use eframe::egui::{Response, Ui};

use crate::colors;

pub fn red_button(ui: &mut Ui, text: &str, is_light_theme: bool) -> Response {
    let (howered_col, inactive_col, active) = match is_light_theme {
        true => (
            colors::LIGHT_RED_HOWER,
            colors::LIGHT_RED_INACTIVE,
            colors::LIGHT_RED_ACTIVE,
        ),
        false => (
            colors::DARK_RED_HOWER,
            colors::DARK_RED_INACTIVE,
            colors::DARK_RED_ACTIVE,
        ),
    };
    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = inactive_col;
    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = howered_col;
    ui.style_mut().visuals.widgets.active.weak_bg_fill = active;
    ui.scope(|ui| ui.button(text)).response
}

pub fn centered(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let id = ui.id().with("_centerer");
        let last_width: Option<f32> = ui.memory_mut(|mem| mem.data.get_temp(id));
        if let Some(last_width) = last_width {
            ui.add_space((ui.available_width() - last_width) / 2.0);
        }
        let res = ui
            .scope(|ui| {
                add_contents(ui);
            })
            .response;
        let width = res.rect.width();
        ui.memory_mut(|mem| mem.data.insert_temp(id, width));

        // Repaint if width changed
        match last_width {
            None => ui.ctx().request_repaint(),
            Some(last_width) if last_width != width => ui.ctx().request_repaint(),
            Some(_) => {}
        }
    });
}
