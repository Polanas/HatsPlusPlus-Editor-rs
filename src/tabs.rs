use std::fmt::Display;
use std::ops::RangeInclusive;
use std::rc::Rc;
use std::sync::Mutex;

use std::cell::RefCell;

use bevy_math::IVec2;
use eframe::egui::{
    self, include_image, Button, CollapsingHeader, Color32, Grid, Image, Key, Layout, RichText,
    ScrollArea, TextureFilter, TextureWrapMode, Vec2,
};
use eframe::emath::Numeric;
use eframe::glow::Context;
use egui_dnd::DragDropItem;

use crate::animations::{AnimType, Frame};
use crate::shader::Shader;
use crate::sprite_drawer::{AnimChangeBehaviour, SpriteDrawer};
use crate::texture_reloader::TextureReloader;
use crate::ui_text::UiText;
use crate::{
    animation_window, animations, frames_from_range, hats, prelude::*, texture_reloader,
    AnimationWindowAction, AppConfig,
};

use eframe::egui::{DragValue, Ui};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex, TabViewer};
use num_traits::ToPrimitive;

use crate::animation_window::{AnimationWindow, AnimationWindowFrameData, TEXTURES_SCALE_FACTOR};
use crate::egui_utils;
use crate::event_bus::EventBus;
use crate::frames_from_range::frames_from_range;
use crate::hats::{
    AbstractHat, Hat, HatElementId, HatType, LinkFrameState, LoadHat, DEFAULT_AUTO_SPEED,
    DEFAULT_PET_DISTANCE, DEFAULT_PET_SPEED,
};
use crate::hats::{Extra, FlyingPet, WalkingPet, Wereable, Wings};
use crate::renderer::Renderer;

pub enum NewHatEvent {
    Opened(std::path::PathBuf),
    New,
}

pub static HAT_EVENT_BUS: Mutex<EventBus<NewHatEvent>> = Mutex::new(EventBus::new());

pub enum HomeUIResponce {
    NewHat(NewHatEvent),
    NewHelpTab,
}

//TODO: add hower text to stuff
//fix hats saving with size when they shouldnt
//wereable:❌
//flyingpet:❌
//walkingpet:❌
//wings:❌
//preview:❌
//animations:❌
pub struct Tabs {
    pub dock_state: DockState<Tab>,
    pub hat_tabs_counter: usize,
}

impl Tabs {
    pub fn new(home_name: String) -> Self {
        let mut dock_state = DockState::new(vec![Tab::new_home(home_name)]);
        dock_state.set_focused_node_and_surface((SurfaceIndex(0), NodeIndex(0)));
        Self {
            dock_state,
            hat_tabs_counter: 1,
        }
    }
    pub fn ui(&mut self, ui: &mut Ui, frame_data: FrameData) {
        let mut added_nodes = vec![];
        let mut tab_viewer = MyTabViewer {
            added_nodes: &mut added_nodes,
            frame_data,
        };
        DockArea::new(&mut self.dock_state)
            .show_add_buttons(true)
            .show_inside(ui, &mut tab_viewer);
        if tab_viewer.frame_data.new_help_tab {
            self.open_help_tab(&tab_viewer.frame_data.ui_text);
        }
        for (surface, node) in added_nodes {
            let tab = Tab::new(format!("Hat {0}", self.hat_tabs_counter), Hat::default());
            self.dock_state
                .set_focused_node_and_surface((surface, node));
            self.dock_state.push_to_focused_leaf(tab);
            self.hat_tabs_counter += 1;
        }
    }

    pub fn open_help_tab(&mut self, ui_text: &UiText) {
        self.dock_state
            .push_to_focused_leaf(Tab::new_help(ui_text.get("Q&A")));
    }

    pub fn open_home_tab(&mut self, ui_text: &UiText) {
        self.dock_state
            .push_to_focused_leaf(Tab::new_home(ui_text.get("Home")));
    }
}

#[derive(Debug, Clone, Copy)]
enum ExamplesState {
    Pressed,
    Released,
    Down,
    None,
}
pub struct HelpTabData {
    state: ExamplesState,
    pub example_1: SpriteDrawer,
    pub example_2: SpriteDrawer,
    pub example_3: SpriteDrawer,
    pub example_4: SpriteDrawer,
}

impl HelpTabData {
    pub fn new(frame_data: &FrameData) -> Self {
        fn chest_drawer(frame_data: &FrameData) -> SpriteDrawer {
            SpriteDrawer::new(
                frame_data.gl,
                "images/linked_state_example_2.png",
                TEXTURES_SCALE_FACTOR / 2.0,
                frame_data.shader.clone(),
                IVec2::new(32, 32),
            )
        }
        fn fridge_drawer(frame_data: &FrameData) -> SpriteDrawer {
            SpriteDrawer::new(
                frame_data.gl,
                "images/interactive_fridge.png",
                TEXTURES_SCALE_FACTOR / 2.0,
                frame_data.shader.clone(),
                IVec2::new(32, 32),
            )
        }
        let mut example_1 = fridge_drawer(frame_data);
        let fridge_anims = [
            Animation::new(AnimType::OnReleaseQuack, 3, true, frames_from_range(0, 49)),
            Animation::new(AnimType::OnPressQuack, 3, true, frames_from_range(50, 99)),
        ];
        for anim in &fridge_anims {
            example_1.add_animation(anim.clone());
        }
        let mut example_2 = fridge_drawer(frame_data);
        for anim in &fridge_anims {
            example_2.add_animation(anim.clone());
        }

        let chest_anims = [
            Animation::new(AnimType::OnDefault, 4, false, frames_from_range(0, 0)),
            Animation::new(AnimType::OnReleaseQuack, 4, false, frames_from_range(7, 0)),
            Animation::new(AnimType::OnPressQuack, 4, false, frames_from_range(0, 7)),
        ];
        let mut example_3 = chest_drawer(frame_data);
        let mut example_4 = chest_drawer(frame_data);
        for anim in &chest_anims {
            example_3.add_animation(anim.clone());
        }
        for anim in &chest_anims {
            example_4.add_animation(anim.clone());
        }
        Self {
            example_1,
            example_2,
            example_3,
            example_4,
            state: ExamplesState::None,
        }
    }
    pub fn update(&mut self, ui: &mut Ui) {
        use ExamplesState as State;

        ui.input(|input| {
            let pressed_e = input.key_pressed(Key::E);
            let down_e = input.key_down(Key::E);

            self.state = match self.state {
                State::None => {
                    if pressed_e {
                        State::Pressed
                    } else {
                        State::None
                    }
                }
                State::Pressed => {
                    if down_e {
                        State::Down
                    } else {
                        State::Released
                    }
                }
                State::Released => {
                    if pressed_e {
                        State::Pressed
                    } else {
                        State::None
                    }
                }
                State::Down => {
                    if down_e {
                        State::Down
                    } else {
                        State::Released
                    }
                }
            }
        });
        match self.state {
            State::Pressed => {
                self.example_1
                    .set_anim(AnimType::OnPressQuack, AnimChangeBehaviour::Keep);
                self.example_2
                    .set_anim(AnimType::OnPressQuack, AnimChangeBehaviour::Reset);
                self.example_3
                    .set_anim(AnimType::OnPressQuack, AnimChangeBehaviour::Reset);
                self.example_4
                    .set_anim(AnimType::OnPressQuack, AnimChangeBehaviour::Reverse);
            }
            State::Released => {
                self.example_1
                    .set_anim(AnimType::OnReleaseQuack, AnimChangeBehaviour::Keep);
                self.example_2
                    .set_anim(AnimType::OnReleaseQuack, AnimChangeBehaviour::Reset);
                self.example_3
                    .set_anim(AnimType::OnReleaseQuack, AnimChangeBehaviour::Reset);
                self.example_4
                    .set_anim(AnimType::OnReleaseQuack, AnimChangeBehaviour::Reverse);
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub enum TabType {
    Regular,
    Home,
    Help,
}

#[derive(Debug)]
pub struct TabInner {
    pub title: String,
    pub hat: Hat,
    pub tab_type: TabType,
    pub selected_hat_id: Option<HatElementId>,
    pub renderer: Option<Renderer>,
    pub animation_window: AnimationWindow,
    pub keep_metapixels: bool,
}

#[derive(Debug)]
pub struct Tab {
    pub inner: RefCell<TabInner>,
}

impl Tab {
    pub fn new(title: String, hat: Hat) -> Self {
        let inner = RefCell::new(TabInner {
            title,
            hat,
            tab_type: TabType::Regular,
            selected_hat_id: None,
            renderer: None,
            animation_window: AnimationWindow::new(),
            keep_metapixels: true,
        });
        Self { inner }
    }

    pub fn new_help(title: String) -> Self {
        let inner = RefCell::new(TabInner {
            title,
            hat: Hat::default(),
            tab_type: TabType::Help,
            selected_hat_id: None,
            renderer: None,
            animation_window: AnimationWindow::new(),
            keep_metapixels: true,
        });
        Self { inner }
    }

    pub fn new_home(title: String) -> Self {
        let inner = RefCell::new(TabInner {
            title,
            hat: Hat::default(),
            tab_type: TabType::Home,
            selected_hat_id: None,
            renderer: None,
            animation_window: AnimationWindow::new(),
            keep_metapixels: true,
        });
        Self { inner }
    }
}

#[derive(Default)]
struct AnimationChanges {
    added: Option<AnimType>,
    removed: Option<AnimType>,
}

impl AnimationChanges {
    fn new(added: Option<AnimType>, removed: Option<AnimType>) -> Self {
        Self { added, removed }
    }
}

pub struct FrameData<'a> {
    pub texture_reloader: &'a mut TextureReloader,
    pub time: f32,
    pub anim_window_action: AnimationWindowAction,
    pub gl: &'a Context,
    pub shader: Shader,
    pub hertz: f32,
    pub ui_text: Rc<UiText>,
    pub config: &'a mut AppConfig,
    pub new_help_tab: bool,
    pub help_data: &'a mut Option<HelpTabData>,
}

pub struct MyTabViewer<'a> {
    added_nodes: &'a mut Vec<(SurfaceIndex, NodeIndex)>,
    frame_data: FrameData<'a>,
}

fn ivec2_ui<Num: Numeric>(
    ui: &mut Ui,
    vec: &mut IVec2,
    range_x: RangeInclusive<Num>,
    range_y: RangeInclusive<Num>,
    text: &str,
) {
    ui.horizontal(|ui| {
        ui.label("X:");
        ui.add(DragValue::new(&mut vec.x).speed(0.2).clamp_range(range_x));
        ui.label("Y:");
        ui.add(DragValue::new(&mut vec.y).clamp_range(range_y));
        ui.label(text);
    });
}

impl MyTabViewer<'_> {
    fn remove_element_ui(&mut self, ui: &mut Ui) -> bool {
        egui_utils::red_button(
            ui,
            &self.frame_data.ui_text.get("25"),
            self.frame_data.config.is_light_theme(),
        )
        .clicked()
    }
    fn draw_extra_hat_ui(&mut self, ui: &mut Ui, inner: &mut TabInner) {
        let hat = &mut inner.hat;
        let extra = hat.extra_mut().unwrap();
        let id = extra.id();
        let mut path = None;
        let mut remove = false;
        ScrollArea::new([true, true])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                ui.allocate_space((ui.available_width(), 1.0).into());
                ui.heading("Extra hat").on_hover_ui(|ui| {
                    ScrollArea::new([true, true])
                        .show(ui, |ui| {
                            ui.label("Extra hat is an optional element that is useful when you want you wereable hat to have a rock, particles, cape or metapixels. ");
                        });
                });
                ui.horizontal(|ui| {
                    if ui.button("Set texture").clicked() {
                        path = rfd::FileDialog::new().pick_file();
                    }
                    ui.checkbox(&mut inner.keep_metapixels, "Keep metapixels");
                });
                remove = self.remove_element_ui(ui);
                ivec2_ui(
                    ui,
                    &mut extra.base_mut().frame_size,
                    hats::MIN_FRAME_SIZE..=hats::MAX_EXTRA_HAT_SIZE.x,
                    hats::MIN_FRAME_SIZE..=hats::MAX_EXTRA_HAT_SIZE.y,
                    "Frame Size",
                );
            });
        let _: Option<()> = try {
            let path = path?;
            if !inner.keep_metapixels {
                let new_hat = Extra::load_from_path(path, self.frame_data.gl).ok()?;
                inner.selected_hat_id = Some(new_hat.base().id);
                self.frame_data
                    .texture_reloader
                    .add_texture(&new_hat.texture().unwrap().clone());
                hat.replace_element(id, new_hat);
            } else {
                let texture = extra.texture_mut()?;
                let old_program = texture.native();
                texture.replace_from_path(self.frame_data.gl, path);
                self.frame_data
                    .texture_reloader
                    .update_texture_program(old_program, texture.native());
            }
        };
        if remove {
            hat.remove_element(id);
            inner.selected_hat_id = None;
        }
    }
    fn draw_wings_ui(&mut self, ui: &mut Ui, inner: &mut TabInner) {
        let hat = &mut inner.hat;
        let wings = hat.wings_mut().unwrap();
        let id = wings.id();
        let mut path = None;
        let mut remove = false;
        ScrollArea::new([true, true])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                let frames_amount = wings.frames_amount();
                ui.allocate_space((ui.available_width(), 1.0).into());
                ui.heading("Wings");
                ui.horizontal(|ui| {
                    if ui.button("Set texture").clicked() {
                        path = rfd::FileDialog::new().pick_file();
                    }
                    ui.checkbox(&mut inner.keep_metapixels, "Keep metapixels");
                });
                remove = self.remove_element_ui(ui);
                ui.horizontal(|ui| {
                    ui.label("Delay");
                    let mut anim = wings.animations[0].borrow_mut();
                    if ui
                        .add(DragValue::new(&mut anim.delay).clamp_range(1..=255))
                        .changed()
                    {
                        wings.auto_anim_speed = anim.delay;
                    }
                    let plus = Button::new("+").min_size(Vec2::splat(18.0));
                    let minus = Button::new("-").min_size(Vec2::splat(18.0));
                    if ui.add(minus).clicked() {
                        anim.delay -= 1;
                        wings.auto_anim_speed = anim.delay;
                    } else if ui.add(plus).clicked() {
                        anim.delay += 1;
                        wings.auto_anim_speed = anim.delay;
                    }
                    if ui.button("Reset").clicked() {
                        anim.delay = DEFAULT_AUTO_SPEED;
                        wings.auto_anim_speed = DEFAULT_AUTO_SPEED;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Glide frame");
                    ui.add(
                        DragValue::new(&mut wings.auto_glide_frame).clamp_range(1..=frames_amount),
                    );
                    if ui.button("Reset").clicked() {
                        wings.auto_glide_frame = frames_amount as i32;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Idle frame");
                    ui.add(
                        DragValue::new(&mut wings.auto_idle_frame).clamp_range(1..=frames_amount),
                    );
                    if ui.button("Reset").clicked() {
                        wings.auto_idle_frame = 0;
                    }
                });
                ivec2_ui(
                    ui,
                    &mut wings.base_mut().frame_size,
                    hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                    hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                    "Frame Size",
                );
                ivec2_ui(
                    ui,
                    &mut wings.crouch_offset,
                    0..=255,
                    0..=255,
                    "Crouch offset",
                );
                ivec2_ui(
                    ui,
                    &mut wings.ragdoll_offset,
                    0..=255,
                    0..=255,
                    "Ragdoll offset",
                );
                ivec2_ui(
                    ui,
                    &mut wings.slide_offset,
                    0..=255,
                    0..=255,
                    "Slide offset",
                );
                ivec2_ui(
                    ui,
                    &mut wings.general_offset,
                    0..=255,
                    0..=255,
                    "Global offset",
                );
            });
        let _: Option<()> = try {
            let path = path?;
            if !inner.keep_metapixels {
                let new_hat = Wings::load_from_path(path, self.frame_data.gl).ok()?;
                inner.selected_hat_id = Some(new_hat.base().id);
                hat.replace_element(id, new_hat);
            } else {
                let texture = wings.texture_mut()?;
                let old_program = texture.native();
                texture.replace_from_path(self.frame_data.gl, path);
                self.frame_data
                    .texture_reloader
                    .update_texture_program(old_program, texture.native());
            }
        };
        if remove {
            hat.remove_element(id);
            inner.selected_hat_id = None;
        }
    }
    fn draw_preview_ui(&mut self, ui: &mut Ui, inner: &mut TabInner) {
        let text = &self.frame_data.ui_text;
        let hat = &mut inner.hat;
        let id = hat.preview().unwrap().id();
        ui.heading(text.get("23"));
        let path = if ui.button(text.get("24")).clicked() {
            rfd::FileDialog::new().pick_file()
        } else {
            None
        };
        let _: Option<()> = try {
            let path = path?;
            let new_hat = Preview::load_from_path(path, self.frame_data.gl).ok()?;
            inner.selected_hat_id = Some(new_hat.base().id);
            hat.replace_element(id, new_hat);
        };
        if self.remove_element_ui(ui) {
            hat.remove_element(id);
            inner.selected_hat_id = None;
        }
    }
    fn draw_flying_pet_ui(&mut self, ui: &mut Ui, inner: &mut TabInner, id: HatElementId) {
        let hat = &mut inner.hat;
        let flying_pet: &mut FlyingPet =
            hat.element_from_id_mut(id).unwrap().downcast_mut().unwrap();
        let id = flying_pet.id();
        let mut path = None;
        let mut remove = false;
        ScrollArea::new([true, true])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                ui.allocate_space((ui.available_width(), 1.0).into());
                ui.heading("Flying pet");
                ui.horizontal(|ui| {
                    if ui.button("Set texture").clicked() {
                        path = rfd::FileDialog::new().pick_file();
                    }
                    ui.checkbox(&mut inner.keep_metapixels, "Keep metapixels");
                });
                remove = self.remove_element_ui(ui);
                ivec2_ui(
                    ui,
                    &mut flying_pet.base_mut().frame_size,
                    hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                    hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                    "Frame Size",
                );
                ui.horizontal(|ui| {
                    ui.label("Distance");
                    ui.add(DragValue::new(&mut flying_pet.pet_base.distance).clamp_range(0..=255));
                    if ui.button("Reset").clicked() {
                        flying_pet.pet_base.distance = DEFAULT_PET_DISTANCE;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Speed");
                    ui.add(DragValue::new(&mut flying_pet.speed).clamp_range(0..=255));
                    if ui.button("Reset").clicked() {
                        flying_pet.speed = DEFAULT_PET_SPEED;
                    }
                });
                ui.checkbox(&mut flying_pet.pet_base.flipped, "Flip");
                ui.checkbox(&mut flying_pet.changes_angle, "Changes angle");
                let anim_changes = self.draw_animations_ui(flying_pet as &mut dyn AbstractHat, ui);
                if let Some(anim) = anim_changes.added {
                    if !flying_pet
                        .pet_base
                        .animations
                        .iter()
                        .any(|h| h.borrow().anim_type == anim)
                    {
                        flying_pet
                            .pet_base
                            .animations
                            .push(RefCell::new(Animation::new(anim, 3, false, vec![])).into());
                    }
                }
                if let Some(anim) = anim_changes.removed {
                    flying_pet
                        .pet_base
                        .animations
                        .retain(|a| a.borrow().anim_type != anim);
                }
            });
        let _: Option<()> = try {
            let path = path?;
            if !inner.keep_metapixels {
                let new_hat = FlyingPet::load_from_path(path, self.frame_data.gl).ok()?;
                inner.selected_hat_id = Some(new_hat.base().id);
                hat.replace_element(id, new_hat);
            } else {
                let texture = flying_pet.texture_mut()?;
                let old_program = texture.native();
                texture.replace_from_path(self.frame_data.gl, path);
                self.frame_data
                    .texture_reloader
                    .update_texture_program(old_program, texture.native());
            }
        };
        if remove {
            hat.remove_element(id);
            inner.selected_hat_id = None;
        }
    }
    fn draw_walking_pet_ui(&mut self, ui: &mut Ui, inner: &mut TabInner, id: HatElementId) {
        let hat = &mut inner.hat;
        let walking_pet: &mut WalkingPet =
            hat.element_from_id_mut(id).unwrap().downcast_mut().unwrap();
        let mut path = None;
        let mut remove = false;
        ScrollArea::new([true, true])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                ui.allocate_space((ui.available_width(), 1.0).into());
                ui.heading("Walking pet");
                ui.heading("Extra hat");
                ui.horizontal(|ui| {
                    if ui.button("Set texture").clicked() {
                        path = rfd::FileDialog::new().pick_file();
                    }
                    ui.checkbox(&mut inner.keep_metapixels, "Keep metapixels");
                });
                remove = self.remove_element_ui(ui);
                ivec2_ui(
                    ui,
                    &mut walking_pet.base_mut().frame_size,
                    hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                    hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                    "Frame Size",
                );
                ui.horizontal(|ui| {
                    ui.label("Distance");
                    ui.add(DragValue::new(&mut walking_pet.pet_base.distance).clamp_range(0..=255));
                    if ui.button("Reset").clicked() {
                        walking_pet.pet_base.distance = DEFAULT_PET_DISTANCE;
                    }
                });
                ui.checkbox(&mut walking_pet.pet_base.flipped, "Flip");
                let anim_changes = self.draw_animations_ui(walking_pet as &mut dyn AbstractHat, ui);
                if let Some(anim) = anim_changes.added {
                    if !walking_pet
                        .pet_base
                        .animations
                        .iter()
                        .any(|h| h.borrow().anim_type == anim)
                    {
                        walking_pet
                            .pet_base
                            .animations
                            .push(RefCell::new(Animation::new(anim, 3, false, vec![])).into());
                    }
                }
                if let Some(anim) = anim_changes.removed {
                    walking_pet
                        .pet_base
                        .animations
                        .retain(|a| a.borrow().anim_type != anim);
                }
            });
        let _: Option<()> = try {
            let path = path?;
            if !inner.keep_metapixels {
                let new_hat = WalkingPet::load_from_path(path, self.frame_data.gl).ok()?;
                inner.selected_hat_id = Some(new_hat.base().id);
                hat.replace_element(id, new_hat);
            } else {
                let texture = walking_pet.texture_mut()?;
                let old_program = texture.native();
                texture.replace_from_path(self.frame_data.gl, path);
                self.frame_data
                    .texture_reloader
                    .update_texture_program(old_program, texture.native());
            }
        };
        if remove {
            hat.remove_element(id);
            inner.selected_hat_id = None;
        }
    }
    fn draw_wereable_hat_ui(&mut self, ui: &mut Ui, inner: &mut TabInner) {
        let hat = &mut inner.hat;
        let wereable = hat.wereable_mut().unwrap();
        let id = wereable.id();
        let mut path = None;
        let mut remove = false;
        ScrollArea::new([true, true])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                ui.allocate_space((ui.available_width(), 1.0).into());
                ui.heading("Wereable hat")
                    .on_hover_ui(|ui| {
                        ui.label("This is the hat your duck will be wearing. Note that at the moment the only avalible frame size is 32x32 pixels.");
                    });
                ui.horizontal(|ui| {
                    if ui.button("Set texture").clicked() {
                        path = rfd::FileDialog::new().pick_file();
                    }
                    ui.checkbox(&mut inner.keep_metapixels, "Keep metapixels");
                });
                remove = self.remove_element_ui(ui);
                ui.checkbox(&mut wereable.strapped_on, "Strapped on");
                // currently there's no point in changing the frame size
                // ivec2_ui(
                //     ui,
                //     &mut wereable.base_mut().frame_size,
                //     hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                //     hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE,
                //     "Frame Size",
                // );
                egui::ComboBox::from_label("Quack Frame Link State")
                    .selected_text(format!("{}", wereable.link_frame_state))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut wereable.link_frame_state,
                            LinkFrameState::Default,
                            "None",
                        );
                        ui.selectable_value(
                            &mut wereable.link_frame_state,
                            LinkFrameState::Saved,
                            "Saved",
                        );
                        ui.selectable_value(
                            &mut wereable.link_frame_state,
                            LinkFrameState::Inverted,
                            "Inverted",
                        );
                    });
                let mut spawn_animation = wereable
                    .on_spawn_animation
                    .unwrap_or(AnimType::Unspecified);
                egui::ComboBox::from_label("Spawn animation")
                    .selected_text(
                        wereable
                            .on_spawn_animation
                            .map(|anim| anim.to_string())
                            .unwrap_or("None".to_owned()),
                    )
                    .show_ui(ui, |ui| {
                        for anim in &wereable.animations {
                            let anim = anim.borrow();
                            ui.selectable_value(
                                &mut spawn_animation,
                                anim.anim_type,
                                anim.anim_type.to_string(),
                            );
                        }
                    });
                if !matches!(spawn_animation, AnimType::Unspecified) {
                    wereable.on_spawn_animation = Some(spawn_animation);
                }
                if !wereable
                    .animations
                    .iter()
                    .any(|a| a.borrow().anim_type == spawn_animation)
                {
                    wereable.on_spawn_animation = None;
                }
                let anim_changes = self.draw_animations_ui(wereable as &mut dyn AbstractHat, ui);
                if let Some(anim) = anim_changes.added {
                    if !wereable.animations.iter().any(|h| h.borrow().anim_type == anim) {
                        wereable
                            .animations
                            .push(RefCell::new(Animation::new(anim, 3, false, vec![])).into());
                    }
                }
                if let Some(anim) = anim_changes.removed {
                    wereable.animations.retain(|a| a.borrow().anim_type != anim);
                }
            });
        let _: Option<()> = try {
            let path = path?;
            if !inner.keep_metapixels {
                let new_hat = Wereable::load_from_path(path, self.frame_data.gl).ok()?;
                inner.selected_hat_id = Some(new_hat.base().id);
                hat.replace_element(id, new_hat);
            } else {
                let texture = wereable.texture_mut()?;
                let old_program = texture.native();
                texture.replace_from_path(self.frame_data.gl, path);
                self.frame_data
                    .texture_reloader
                    .update_texture_program(old_program, texture.native());
            }
        };
        if remove {
            hat.remove_element(id);
            inner.selected_hat_id = None;
        }
    }
    fn draw_animations_ui(&mut self, hat: &mut dyn AbstractHat, ui: &mut Ui) -> AnimationChanges {
        let frames_amount = hat.frames_amount();
        let mut anim_to_delete = None;
        let mut anim_to_add = None;
        let Some(avalible_anims) = animations::avalible_animations(hat.base().hat_type) else {
            return AnimationChanges::default();
        };
        let can_add_animations =
            avalible_anims.len() != hat.animations().map(|a| a.len()).unwrap_or(0);
        let open = match can_add_animations {
            true => None,
            false => Some(false),
        };
        CollapsingHeader::new("Add an animation")
            .open(open)
            .enabled(can_add_animations)
            .show(ui, |ui| {
                for anim in avalible_anims {
                    if ui.button(anim.to_string()).clicked() {
                        anim_to_add = Some(anim);
                    }
                }
            });
        ui.collapsing("Animations", |ui| {
            if let Some(anims) = hat.animations_mut() {
                for anim in anims {
                    let mut anim = anim.borrow_mut();
                    egui::CollapsingHeader::new(anim.anim_type.to_string()).show(ui, |ui| {
                        Grid::new("grid").show(ui, |ui| {
                            ui.label("Delay");
                            ui.horizontal(|ui| {
                                ui.add(DragValue::new(&mut anim.delay).clamp_range(1..=255));
                                let plus = Button::new("+").min_size(Vec2::splat(18.0));
                                let minus = Button::new("-").min_size(Vec2::splat(18.0));
                                if ui.add(minus).clicked() {
                                    anim.delay -= 1;
                                } else if ui.add(plus).clicked() {
                                    anim.delay += 1;
                                }
                            });
                            ui.end_row();
                            ui.label("Looping");
                            ui.checkbox(&mut anim.looping, "");
                        });
                        let mut delete_frame_index = None;
                        let mut add_frame_index = None;
                        egui_dnd::dnd(ui, "my_dnd").show_vec(
                            &mut anim.frames,
                            |ui, item: &mut Frame, handle, state| {
                                ui.push_id(item.id().value(), |ui| {
                                    ui.horizontal(|ui| {
                                        handle.ui(ui, |ui| {
                                            ui.label((item.value + 1).to_string());
                                            if ui.button("+").clicked() {
                                                add_frame_index = Some(state.index);
                                            }
                                            if ui.button("X").clicked() {
                                                delete_frame_index = Some(state.index);
                                            }
                                        });
                                    });
                                });
                            },
                        );
                        if let Some(index) = delete_frame_index {
                            anim.frames.remove(index);
                        }
                        if let Some(index) = add_frame_index {
                            let frame = anim.frames[index].clone();
                            anim.frames.insert(index, frame);
                        }

                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut anim.new_frame)).changed();
                            if ui.button("Add Frame").clicked()
                                && (1..=frames_amount)
                                    .contains(&anim.new_frame.to_u32().unwrap_or(0))
                            {
                                let frame = anim.new_frame - 1;
                                anim.frames.push(frame.into());
                                anim.new_frame += 1;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Start:");
                            ui.add(
                                egui::DragValue::new(&mut anim.new_range_start)
                                    .clamp_range(0..=i32::MAX),
                            )
                            .changed();
                            ui.label("End:");
                            ui.add(
                                egui::DragValue::new(&mut anim.new_range_end)
                                    .clamp_range(0..=i32::MAX),
                            )
                            .changed();
                            ui.label(" ");
                            if ui.button("Set Frame Range").clicked() {
                                let range_start = (anim.new_range_start - 1).max(0);
                                let range_end =
                                    (anim.new_range_end - 1).clamp(0, frames_amount as i32 - 1);
                                anim.frames = frames_from_range(range_start, range_end);
                            }
                        });
                        if ui.button("Clear Frames").clicked() {
                            anim.frames.clear();
                        }
                        if egui_utils::red_button(
                            ui,
                            "Delete",
                            self.frame_data.config.is_light_theme(),
                        )
                        .clicked()
                        {
                            anim_to_delete = Some(anim.anim_type);
                        }
                    });
                }
            }
        });
        AnimationChanges::new(anim_to_add.copied(), anim_to_delete)
    }

    fn help_ui(&mut self, ui: &mut Ui) {
        macro_rules! texture {
            ($ui:ident, $path:expr) => {
                let image = Image::new(include_image!($path))
                    .texture_options(egui::TextureOptions {
                        magnification: TextureFilter::Nearest,
                        minification: TextureFilter::Nearest,
                        wrap_mode: TextureWrapMode::ClampToEdge,
                    })
                    .fit_to_original_size(animation_window::TEXTURES_SCALE_FACTOR / 2.0);
                $ui.add(image);
            };
        }
        let frame_data = &mut self.frame_data;
        let help_data = match frame_data.help_data {
            Some(d) => d,
            None => {
                *frame_data.help_data = Some(HelpTabData::new(frame_data));
                &mut frame_data.help_data.as_mut().unwrap()
            }
        };
        help_data.update(ui);
        // ui.horizontal(|ui| {
        //     ui.spacing_mut().item_spacing.x = 50.0;
        //     help_data
        //         .example_1
        //         .draw(ui, frame_data.time, frame_data.hertz);
        //     help_data
        //         .example_2
        //         .draw(ui, frame_data.time, frame_data.hertz);
        //     help_data
        //         .example_3
        //         .draw(ui, frame_data.time, frame_data.hertz);
        //     help_data
        //         .example_4
        //         .draw(ui, frame_data.time, frame_data.hertz);
        // });
        //
        let text = &frame_data.ui_text;
        ui.collapsing(text.get("15"), |ui| {
            ui.label(text.get("16"));
        });
        ui.collapsing(text.get("4"), |ui| {
            ui.label(text.get("5"));
            ui.label(format!("• {}", text.get("Preview")));
            ui.label(format!("• {}", text.get("Wereable")));
            ui.label(format!("• {}", text.get("Wings")));
            ui.label(format!("• {}", text.get("Extra")));
            ui.label(format!("• {}", text.get("Flying pet")));
            ui.label(format!("• {}", text.get("Walking pet")));
        });
        ui.collapsing(text.get("1"), |ui| {
            ui.label(text.get("2"));
            ui.collapsing("Preview", |ui| {
                ui.label(text.get("6"));
                texture!(ui, "../images/preview_example.png");
            });
            ui.collapsing("Wearable", |ui| {
                ui.label(text.get("3"));
                texture!(ui, "../images/wereable_example.png");
            });
            ui.collapsing("Extra", |ui| {
                ui.label(text.get("A1"));
                texture!(ui, "../images/extra_hat_example.png");
                ui.label(text.get("A1_2"));
                texture!(ui, "../images/extra_hat_example_2.png");
                ui.label(text.get("A1_3"));
                texture!(ui, "../images/extra_hat_example_3.png");
                ui.label(text.get("A1_4"));
                texture!(ui, "../images/extra_hat_example_4.png");
                ui.label(text.get("A1_5"));
                texture!(ui, "../images/extra_hat_example_5.png");
                ui.label(text.get("A1_6"));
                texture!(ui, "../images/extra_hat_example_6.png");
            });
            ui.collapsing(text.get("7"), |ui| {
                ui.label(text.get("8"));
                texture!(ui, "../images/walkingpet_example.png");
                ui.label(text.get("8.1"));
                texture!(ui, "../images/flying_pet_example.png");
            });
            ui.collapsing(text.get("Wings"), |ui| {
                ui.label(text.get("9"));
                texture!(ui, "../images/wings_example.png");
            });
        });
        ui.collapsing(text.get("10"), |ui| {
            ui.heading(text.get("12"));
            ui.label(text.get("11"));
            texture!(ui, "../images/sprite_sheet_example.png");
            ui.heading(text.get("13"));
            ui.label(text.get("14"));
        });
        ui.collapsing(text.get("17"), |ui| {
            ui.label(text.get("18"));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 50.0;
                help_data
                    .example_1
                    .draw(ui, frame_data.time, frame_data.hertz);
                help_data
                    .example_2
                    .draw(ui, frame_data.time, frame_data.hertz);
            });
            ui.label(text.get("21"));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 50.0;
                help_data
                    .example_3
                    .draw(ui, frame_data.time, frame_data.hertz);
                help_data
                    .example_4
                    .draw(ui, frame_data.time, frame_data.hertz);
            });
        });
    }

    fn home_ui(&mut self, ui: &mut Ui) -> Option<HomeUIResponce> {
        let text = &self.frame_data.ui_text;
        ui.heading(text.get("Welcome"));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(text.get("This app"));
            ui.hyperlink_to(
                text.get("HatsPlusPlus"),
                "https://steamcommunity.com/sharedfiles/filedetails/?id=2695242065",
            )
            .on_hover_ui(|ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(text.get("With this 1"));
                    ui.label(RichText::new(text.get("Rooms")).strikethrough());
                });
                ui.label(text.get("With this 2"));
                // ui.label("").strikethrough()
            });
            ui.label(text.get("mod"));
        });
        let mut responce = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(text.get("To start"));
            if ui.link(text.get("Open")).clicked() {
                if let Some(dir_path) = rfd::FileDialog::new().pick_folder() {
                    responce = Some(HomeUIResponce::NewHat(NewHatEvent::Opened(dir_path)));
                }
            }
            ui.label(text.get("or create"));
            if ui.link(text.get("New")).clicked() {
                responce = Some(HomeUIResponce::NewHat(NewHatEvent::New));
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(text.get("For more"));
            if ui.link(text.get("Q&A")).clicked() {
                responce = Some(HomeUIResponce::NewHelpTab);
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.heading(text.get("Enjoy")).on_hover_text("I love you.");
            ui.heading(
                egui::RichText::new("♥")
                    .heading()
                    .color(Color32::from_rgb(242, 56, 56)),
            )
            .on_hover_text(text.get("I love you"));
        });
        ui.with_layout(Layout::left_to_right(egui::Align::default()), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(text.get("By the way"));
            ui.label(text.get("hower"))
                .on_hover_text(text.get("Now thats"));
            ui.label(text.get("over something"));
        });
        ui.label(text.get("Oh, and"));
        responce
    }

    fn draw_hat_ui(&mut self, selected_hat_id: HatElementId, inner: &mut TabInner, ui: &mut Ui) {
        let hat_id = inner.selected_hat_id.unwrap();
        match inner.hat.hat_type_by_id(selected_hat_id).unwrap() {
            HatType::Wereable => self.draw_wereable_hat_ui(ui, inner),
            HatType::Wings => self.draw_wings_ui(ui, inner),
            HatType::FlyingPet => self.draw_flying_pet_ui(ui, inner, hat_id),
            HatType::WalkingPet => self.draw_walking_pet_ui(ui, inner, hat_id),
            HatType::Extra => self.draw_extra_hat_ui(ui, inner),
            HatType::Preview => self.draw_preview_ui(ui, inner),
            _ => {}
        };
    }
}

impl TabViewer for MyTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.inner.borrow().title.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let inner: &mut TabInner = &mut tab.inner.borrow_mut();
        match inner.tab_type {
            TabType::Home => {
                if let Some(responce) = self.home_ui(ui) {
                    match responce {
                        HomeUIResponce::NewHat(hat) => (*HAT_EVENT_BUS.lock().unwrap()).send(hat),
                        HomeUIResponce::NewHelpTab => self.frame_data.new_help_tab = true,
                    };
                }
                return;
            }
            TabType::Help => {
                self.help_ui(ui);
                return;
            }
            TabType::Regular => {}
        };
        if inner.selected_hat_id.is_none() {
            if !inner.hat.has_elements() {
                ui.label("Looks like this has is totaly empty! Maybe add an element or two?");
                return;
            }
            let first_id = inner
                .hat
                .iter_all_elements()
                .next()
                .map(|h| h.id())
                .unwrap();
            inner.selected_hat_id = Some(first_id);
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Path: ");
            let Some(path) = &inner.hat.path else {
                ui.label("none. At least for now :)");
                return;
            };
            if ui.link(path.to_string_lossy().to_string()).clicked() {
                std::process::Command::new("xdg-open")
                    .arg(path.as_os_str())
                    .spawn()
                    .unwrap();
            }
        });
        let selected_hat_id = inner.selected_hat_id.unwrap();
        let hat_name = inner.title.clone();
        let selected_hat = inner.hat.element_from_id_mut(selected_hat_id).unwrap();
        let frame_size = selected_hat.base().frame_size;
        let animations = selected_hat.animations().map(|a| a.to_vec());
        if let Some(texture) = selected_hat.texture().cloned() {
            //keep calm and call clone, right?
            inner.animation_window.draw(AnimationWindowFrameData {
                ui,
                shader: self.frame_data.shader.clone(),
                hertz: self.frame_data.hertz,
                animations,
                texture: texture.clone(),
                frame_size,
                hat_name,
                anim_window_action: self.frame_data.anim_window_action,
                time: self.frame_data.time,
            });
        }
        self.draw_hat_ui(selected_hat_id, inner, ui);
    }

    fn on_add(&mut self, surface: egui_dock::SurfaceIndex, node: egui_dock::NodeIndex) {
        self.added_nodes.push((surface, node))
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        let hat = &mut tab.inner.borrow_mut().hat;
        let latest_hats = &mut self.frame_data.config.latest_hats;
        if let Some(path) = &hat.path {
            if !latest_hats.iter().any(|p| p == path) {
                latest_hats.push(path.clone());
            }
        }
        hat.delete_textures(self.frame_data.gl);
        true
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
