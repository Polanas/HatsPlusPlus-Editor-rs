#![feature(once_cell_get_mut)]
#![feature(try_blocks)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod animation_window;
mod animations;
mod colors;
mod egui_utils;
mod event_bus;
mod file_utils;
mod frames_from_range;
mod hat_utils;
mod hats;
mod image_extensions;
mod is_range;
mod metapixels;
mod prelude;
mod renderer;
mod shader;
mod shader_reloader;
mod shortcuts;
mod sprite;
mod tabs;
mod texture;
mod texture_reloader;
mod ui_text;

#[macro_use]
extern crate num_derive;

use anyhow::{bail, Result};
use eframe::egui::{
    vec2, Button, CentralPanel, CollapsingHeader, FontDefinitions, Id, KeyboardShortcut, RichText,
    ViewportBuilder,
};
use eframe::glow::NativeBuffer;
use eframe::glow::{self, SAMPLE_ALPHA_TO_COVERAGE};
use eframe::{
    egui::{self, Ui},
    glow::{Context, HasContext, NativeVertexArray},
    NativeOptions,
};
use file_utils::FileStemString;
use hats::{AbstractHat, Extra, FlyingPet, Hat, LoadHat, Preview, WalkingPet, Wereable, Wings};
use num_traits::CheckedSub;
use renderer::{Renderer, ScreenUpdate};
use serde::{Deserialize, Serialize};
use shader::Shader;
use shader_reloader::ShaderReloader;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::RwLock;
use std::time::SystemTime;
use tabs::{Tab, Tabs};
use texture_reloader::TextureReloader;
use ui_text::{Language, UiText};

const HERTZ_MAGIC_NUMBER: u32 = 3;
pub static VERTEX_BUFFER: RwLock<Option<NativeBuffer>> = RwLock::new(None);
pub static VERTEX_ARRAY: RwLock<Option<NativeVertexArray>> = RwLock::new(None);

#[derive(Deserialize, Serialize, Clone, Debug, Copy)]
pub enum Theme {
    Latte,
    Frappe,
    Macchiato,
    Mocha,
}

impl Theme {
    pub fn catppuccin(&self) -> catppuccin_egui::Theme {
        match self {
            Theme::Latte => catppuccin_egui::LATTE,
            Theme::Frappe => catppuccin_egui::FRAPPE,
            Theme::Macchiato => catppuccin_egui::MACCHIATO,
            Theme::Mocha => catppuccin_egui::MOCHA,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AppConfig {
    pub language: Language,
    pub theme: Theme,
    pub latest_hats: Vec<PathBuf>,
}

impl AppConfig {
    pub fn is_light_theme(&self) -> bool {
        matches!(self.theme, Theme::Latte)
    }
    pub fn remove_invalid_latest(&mut self) {
        self.latest_hats.retain(|path| path.exists());
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum AnimationWindowAction {
    #[default]
    None,
    Pause,
    IncreaseFrame,
    DecreaseFrame,
}

pub struct FrameData<'a> {
    anim_window_action: AnimationWindowAction,
    gl: &'a Context,
    shader: Shader,
    time: f32,
    hertz: u32,
    ui_text: Rc<UiText>,
    config: &'a mut AppConfig,
}

impl<'a> FrameData<'a> {
    pub fn new(
        gl: &'a Context,
        shader: Shader,
        time: f32,
        hertz: u32,
        ui_text: Rc<UiText>,
        config: &'a mut AppConfig,
        anim_window_action: AnimationWindowAction,
    ) -> Self {
        Self {
            config,
            gl,
            shader,
            time,
            hertz,
            ui_text,
            anim_window_action,
        }
    }
}

trait ShortcutPressed {
    fn shortcut_pressed(&mut self, shortcut: KeyboardShortcut) -> bool;
}

impl ShortcutPressed for Ui {
    fn shortcut_pressed(&mut self, shortcut: KeyboardShortcut) -> bool {
        self.input_mut(|input| input.consume_shortcut(&shortcut))
    }
}

struct MyEguiApp {
    config: Rc<AppConfig>,
    ui_text: Rc<UiText>,
    texture_reloader: TextureReloader,
    shader_reloader: ShaderReloader,
    animation_shader: Shader,
    tabs: Tabs,
    last_time: SystemTime,
    current_time: SystemTime,
    time: f32,
    hertz: u32,
    calculated_hertz: bool,
}

impl MyEguiApp {
    fn calculate_hertz(&mut self) {
        if self.time < 1.0 {
            self.hertz += 1;
        } else if !self.calculated_hertz {
            self.calculated_hertz = true;
            self.hertz -= HERTZ_MAGIC_NUMBER;
        }
    }

    fn delta_time(&self) -> f32 {
        let duration = self
            .current_time
            .duration_since(self.last_time)
            .unwrap_or_default();
        duration.as_secs_f32()
    }

    fn execute_shortcuts(&mut self, gl: &Context, ui: &mut Ui) {
        if ui.shortcut_pressed(shortcuts::OPEN) {
            let _ = self.open_hat_with_dialog(gl);
        } else if ui.shortcut_pressed(shortcuts::SAVE) {
            self.save_hat();
        } else if ui.shortcut_pressed(shortcuts::NEW) {
            self.add_new_hat();
        } else if ui.shortcut_pressed(shortcuts::HOME) {
            self.open_home_tab();
        } else if ui.shortcut_pressed(shortcuts::SAVE_AS) {
            self.save_hat_as();
        }
    }

    fn draw_hat_menu(&mut self, ctx: &egui::Context, gl: &Context, ui: &mut Ui) {
        let text = self.ui_text.clone();
        let last_tab = self.last_interacted_tab();
        let has_path = last_tab
            .map(|tab| tab.inner.borrow().hat.path.is_some())
            .unwrap_or(false);
        let has_elements = last_tab
            .map(|tab| tab.inner.borrow().hat.has_elements())
            .unwrap_or(false);
        ui.menu_button(text.get("Hat"), |ui| {
            if ui
                .add(self.button_shortcut(ctx, &text.get("New1"), shortcuts::NEW))
                .clicked()
            {
                self.add_new_hat();
                ui.close_menu();
            } else if ui
                .add(self.button_shortcut(ctx, &text.get("Open1"), shortcuts::OPEN))
                .clicked()
            {
                let _ = self.open_hat_with_dialog(gl);
                ui.close_menu();
            } else if ui
                .add_enabled(
                    has_path,
                    self.button_shortcut(ctx, &text.get("Save"), shortcuts::SAVE),
                )
                .clicked()
            {
                self.save_hat();
                ui.close_menu();
            } else if ui
                .add_enabled(
                    has_elements,
                    self.button_shortcut(ctx, &text.get("Save as"), shortcuts::SAVE_AS),
                )
                .clicked()
            {
                self.save_hat_as();
                ui.close_menu()
            }
            ui.collapsing(text.get("Recent"), |ui| {
                let mut recent_hat_path = None;
                for path in self.config.latest_hats.iter().rev() {
                    if !path.exists() {
                        continue;
                    }
                    let Some(name) = path.file_stem_string() else {
                        continue;
                    };
                    if ui.button(name).clicked() {
                        recent_hat_path = Some(path.clone());
                        ui.close_menu();
                    }
                }
                if let Some(path) = recent_hat_path {
                    if self.config.latest_hats.iter().any(|p| *p == path) {
                        let _ = self.open_hat(gl, &path);
                    }
                }
                if !self.config.latest_hats.is_empty()
                    && egui_utils::red_button(ui, "Clear", self.config.is_light_theme()).clicked()
                {
                    Rc::get_mut(&mut self.config).unwrap().latest_hats.clear();
                }
            });
        });

        ui.menu_button(text.get("Elements"), |ui| {
            self.draw_elements_menu(ui, gl);
        });
        ui.menu_button(text.get("Settings"), |ui| {
            self.draw_setings_menu(ui, gl);
        });
        ui.menu_button(text.get("Other"), |ui| {
            if ui.button(text.get("Open Home")).clicked() {
                self.open_home_tab();
                ui.close_menu();
            }
        });
    }

    fn draw_elements_menu(&mut self, ui: &mut Ui, gl: &Context) {
        let text = self.ui_text.clone();
        let Some(last_tab) = self.last_interacted_tab_mut() else {
            return;
        };
        let mut inner = last_tab.inner.borrow_mut();
        // no FUCKING way this works
        macro_rules! try_add_hat {
            ($ui:ident, $get_hat:ident, $hat_type:ident) => {
                if inner.hat.$get_hat().is_none()
                    && $ui
                        .button(hats::HatType::$hat_type.get_display_name(&text))
                        .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Ok(hat) = $hat_type::load_from_path(path, gl) {
                            inner
                                .hat
                                .add_unique_hat(hats::HatType::$hat_type, Box::new(hat));
                            $ui.close_menu();
                        }
                    }
                }
            };
        }
        let is_home = inner.is_home_tab;
        ui.add_enabled_ui(!is_home, |ui| {
            ui.collapsing(text.get("Add"), |ui| {
                try_add_hat!(ui, wereable, Wereable);
                try_add_hat!(ui, wings, Wings);
                try_add_hat!(ui, extra, Extra);
                try_add_hat!(ui, preview, Preview);
                if inner.hat.preview().is_none()
                    && ui
                        .add_enabled(
                            inner.hat.can_add_pets(),
                            Button::new(text.get("Walking pet")),
                        )
                        .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Ok(hat) = WalkingPet::load_from_path(path, gl) {
                            inner.hat.add_pet(Box::new(hat));
                            ui.close_menu();
                        }
                    }
                }
                if inner.hat.preview().is_none()
                    && ui
                        .add_enabled(
                            inner.hat.can_add_pets(),
                            Button::new(text.get("Flying pet")),
                        )
                        .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Ok(hat) = FlyingPet::load_from_path(path, gl) {
                            inner.hat.add_pet(Box::new(hat));
                            ui.close_menu();
                        }
                    }
                }
            });
            ui.add_enabled_ui(inner.hat.has_elements(), |ui| {
                ui.collapsing(text.get("Select"), |ui| {
                    for elements in inner.hat.unique_elemets.values() {
                        let hat_type = elements.base().hat_type;
                        if ui
                            .button(hat_type.get_display_name(text.as_ref()))
                            .clicked()
                        {
                            inner.selected_hat_id =
                                Some(inner.hat.id_from_hat_type(hat_type).unwrap());
                            ui.close_menu();
                            break;
                        }
                    }
                    for pet in inner.hat.pets.iter() {
                        let size = pet.base().hat_area_size;
                        let button_name = format!(
                            "{0} ({1}, {2})",
                            pet.base().hat_type.get_display_name(text.as_ref()),
                            size.x,
                            size.y
                        );
                        if ui.button(button_name).clicked() {
                            inner.selected_hat_id = Some(pet.id());
                            ui.close_menu();
                            break;
                        }
                    }
                })
            })
        });
    }

    fn open_home_tab(&mut self) {
        self.tabs
            .dock_state
            .push_to_focused_leaf(Tab::new_home(format!(
                "{0} {1}",
                self.ui_text.get("Home"),
                self.tabs.home_tabs_counter
            )));
        self.tabs.home_tabs_counter += 1;
    }

    fn last_interacted_tab(&mut self) -> Option<&Tab> {
        self.tabs
            .dock_state
            .find_active_focused()
            .map(|(_, tab)| &*tab)
    }

    fn last_interacted_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs
            .dock_state
            .find_active_focused()
            .map(|(_, tab)| tab)
    }

    fn open_hat_with_dialog(&mut self, gl: &Context) -> Result<()> {
        let path = match rfd::FileDialog::new().pick_folder() {
            Some(path) => path,
            None => bail!("coud not pick file"),
        };
        self.open_hat(gl, &path)
    }

    fn open_hat(&mut self, gl: &Context, dir_path: impl AsRef<Path>) -> Result<()> {
        if self.tabs.dock_state.iter_all_tabs().any(|t| {
            t.1.inner
                .borrow()
                .hat
                .path
                .as_ref()
                .map(|p| p == dir_path.as_ref())
                .unwrap_or_default()
        }) {
            bail!("hat with the same path is already opened");
        }
        let hat = Hat::load(dir_path, gl)?;
        let selected_hat_id = hat.iter_all_elements().next().map(|h| h.id());
        let name = match hat
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|p| p.to_owned())
            .map(|p| p.to_string_lossy().to_string())
        {
            Some(name) => name,
            None => bail!("could not get hat name"),
        };
        hat.add_textures_to_reloader(&mut self.texture_reloader);
        let tab = Tab::new(name, hat);
        let mut inner = tab.inner.borrow_mut();
        if inner.hat.room().is_some() {
            inner.renderer = Some(Renderer::new(
                renderer::RENDERER_SCREEN_SIZE,
                ScreenUpdate::Clear,
                gl,
            ));
        }
        inner.selected_hat_id = selected_hat_id;
        drop(inner);
        self.tabs.dock_state.push_to_focused_leaf(tab);
        Ok(())
    }

    fn regular_tab_name(&mut self) -> String {
        let name = format!("Hat {0}", self.tabs.hat_tabs_counter);
        self.tabs.hat_tabs_counter += 1;
        name
    }

    fn save_hat_as(&mut self) -> Option<()> {
        let dir_path = rfd::FileDialog::new().pick_folder()?;
        let last_tab = self.last_interacted_tab_mut()?;
        last_tab.inner.borrow_mut().hat.path = Some(dir_path.clone());
        let result = last_tab.inner.borrow_mut().hat.save(dir_path);
        result.ok()
    }
    //TODO: change tab's name after save
    fn save_hat(&mut self) -> Option<()> {
        let last_tab = self.last_interacted_tab_mut()?;
        let hat = &mut last_tab.inner.borrow_mut().hat;
        //if save is avalible, the hat has a path
        hat.save(hat.path.as_ref().unwrap()).ok()
    }

    fn button_shortcut(
        &self,
        ctx: &egui::Context,
        text: &str,
        shortcut: KeyboardShortcut,
    ) -> Button<'static> {
        Button::new(text).shortcut_text(ctx.format_shortcut(&shortcut))
    }

    fn add_new_hat(&mut self) {
        let tab = Tab::new(self.regular_tab_name(), Hat::default());
        self.tabs.dock_state.push_to_focused_leaf(tab);
    }

    fn init_opengl_objects(gl: &Context) {
        let vertices: [f32; 12] = [
            -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
        ];
        unsafe {
            let vertices_u8: &[u8] = core::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                vertices.len() * core::mem::size_of::<f32>(),
            );

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            *VERTEX_ARRAY.write().unwrap() = Some(vertex_array);
            gl.bind_vertex_array(Some(vertex_array));

            let vertex_buffer = gl.create_buffer().unwrap();
            *VERTEX_BUFFER.write().unwrap() = Some(vertex_buffer);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertices_u8, glow::STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(
                0,
                2,
                glow::FLOAT,
                false,
                2 * core::mem::size_of::<f32>() as i32,
                0,
            );
            gl.enable_vertex_attrib_array(0);
        }
    }

    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut config: AppConfig = match cc
            .egui_ctx
            .memory_mut(|memory| memory.data.get_persisted(Id::NULL))
        {
            Some(config) => config,
            None => {
                let config = AppConfig {
                    language: Language::English,
                    theme: Theme::Mocha,
                    latest_hats: vec![],
                };
                cc.egui_ctx.memory_mut(|memory| {
                    memory.data.insert_persisted(Id::NULL, config.clone());
                });
                config
            }
        };
        config.remove_invalid_latest();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Caskaydia".to_owned(),
            egui::FontData::from_static(include_bytes!("../font.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Caskaydia".to_owned());
        cc.egui_ctx.set_fonts(fonts);
        catppuccin_egui::set_theme(&cc.egui_ctx, config.theme.catppuccin());
        let language = config.language;
        let gl = cc.gl.as_ref().unwrap().as_ref();
        let mut shader_reloader = ShaderReloader::new();
        let animation_shader =
            Shader::from_path(gl, "src/anim_shader/frag.glsl", "src/anim_shader/vert.glsl")
                .unwrap();
        shader_reloader.add_shader(&animation_shader);
        let ui_text: Rc<UiText> = UiText::new(language, "text.json").into();
        let home_name = ui_text.get("Home");
        MyEguiApp::init_opengl_objects(gl);
        Self {
            ui_text,
            config: config.into(),
            animation_shader,
            shader_reloader,
            texture_reloader: TextureReloader::new(),
            time: 0.0,
            hertz: 0,
            tabs: Tabs::new(home_name),
            last_time: SystemTime::now(),
            current_time: SystemTime::now(),
            calculated_hertz: false,
        }
    }

    fn set_theme(&mut self, ui: &Ui, theme: Theme) {
        Rc::get_mut(&mut self.config).unwrap().theme = theme;
        catppuccin_egui::set_theme(ui.ctx(), theme.catppuccin());
    }

    fn set_language(&mut self, lang: Language) {
        Rc::get_mut(&mut self.config).unwrap().language = lang;
        let lang_data = &self.ui_text.data;
        for tab in self.tabs.dock_state.iter_all_tabs_mut() {
            let mut inner = tab.1.inner.borrow_mut();
            if inner.title == lang_data["ru"]["Home"] || inner.title == lang_data["en"]["Home"] {
                inner.title = match lang {
                    Language::English => lang_data["en"]["Home"].clone(),
                    Language::Russian => lang_data["ru"]["Home"].clone(),
                };
            }
        }
    }

    fn show_hidden_page(&mut self, ui: &mut Ui) {
        if self.tabs.dock_state.iter_all_tabs().count() == 0 {
            ui.label("May I congratulate you on finding this hidden page! As a little present, check out this cute hat 🐱");
            let image_source = egui::include_image!("../cutie.png");
            let image = egui::Image::new(image_source).rounding(20.0);
            ui.add(image).on_hover_text("Ins't it adorable, right?");
        }
    }

    fn on_close(&mut self, ctx: &egui::Context) {
        {
            let config = Rc::get_mut(&mut self.config).unwrap();
            for (_, tab) in self.tabs.dock_state.iter_all_tabs() {
                let hat = &tab.inner.borrow().hat;
                if let Some(path) = &hat.path {
                    if !config.latest_hats.iter().any(|p| p == path) {
                        config.latest_hats.push(path.to_owned());
                    }
                }
            }
            config.remove_invalid_latest();
        }
        ctx.memory_mut(|memory| {
            memory
                .data
                .insert_persisted(Id::NULL, (*self.config).clone());
        });
    }

    fn pre_update(&mut self, ctx: &egui::Context, gl: &Context) {
        self.current_time = SystemTime::now();
        self.time += self.delta_time();
        ctx.request_repaint();
        ctx.set_pixels_per_point(1.5);
        self.texture_reloader.try_reload(gl);
        if cfg!(debug_assertions) {
            self.shader_reloader.try_reload(gl);
        }
        Rc::get_mut(&mut self.ui_text).unwrap().language = self.config.language;
    }

    fn draw_setings_menu(&mut self, ui: &mut Ui, gl: &Context) {
        let text = self.ui_text.clone();
        ui.collapsing(text.get("Theme"), |ui| {
            if ui.button("Latte").clicked() {
                self.set_theme(ui, Theme::Latte);
                ui.close_menu();
            } else if ui.button("Frappé").clicked() {
                self.set_theme(ui, Theme::Frappe);
                ui.close_menu();
            } else if ui.button("Macchiato").clicked() {
                self.set_theme(ui, Theme::Macchiato);
                ui.close_menu();
            } else if ui.button("Mocha").clicked() {
                self.set_theme(ui, Theme::Mocha);
                ui.close_menu();
            }
        });
        ui.collapsing(text.get("Language"), |ui| {
            if ui.button(text.get("English")).clicked() {
                self.set_language(Language::English);
                ui.close_menu();
            } else if ui.button(text.get("Russian")).clicked() {
                self.set_language(Language::Russian);
                ui.close_menu();
            }
        });
    }
    fn animation_window_action(ui: &mut Ui) -> AnimationWindowAction {
        if ui.shortcut_pressed(shortcuts::PAUSE) {
            AnimationWindowAction::Pause
        } else if ui.shortcut_pressed(shortcuts::DECREASE_FRAME) {
            AnimationWindowAction::DecreaseFrame
        } else if ui.shortcut_pressed(shortcuts::INCREASE_FRAME) {
            AnimationWindowAction::IncreaseFrame
        } else {
            AnimationWindowAction::None
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            self.on_close(ctx);
        }
        let gl = frame.gl().unwrap().as_ref();
        self.pre_update(ctx, gl);
        self.calculate_hertz();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());
            egui::menu::bar(ui, |ui| {
                self.draw_hat_menu(ctx, gl, ui);
            });
            self.show_hidden_page(ui);
            let anim_window_action = MyEguiApp::animation_window_action(ui);
            self.tabs.ui(
                ui,
                FrameData::new(
                    gl,
                    self.animation_shader.clone(),
                    self.time,
                    self.hertz,
                    self.ui_text.clone(),
                    Rc::get_mut(&mut self.config).unwrap(),
                    anim_window_action,
                ),
            );
            self.execute_shortcuts(gl, ui);
            let mut hat_event_bus = tabs::HAT_EVENT_BUS.lock().unwrap();
            if let Some(hat_event) = hat_event_bus.read() {
                let _ = match hat_event {
                    tabs::NewHatEvent::Opened(path) => self.open_hat(gl, &path),
                    tabs::NewHatEvent::New => {
                        self.add_new_hat();
                        Ok(())
                    }
                };
            }
        });
        self.last_time = SystemTime::now();
    }
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    let native_opts = NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: ViewportBuilder::default().with_inner_size(vec2(1600.0, 900.0)),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Editor",
        native_opts,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(MyEguiApp::new(cc))
        }),
    );
    Ok(())
}
