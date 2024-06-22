use std::sync::Mutex;

use std::cell::RefCell;

use eframe::egui::{
    self, Button, CollapsingHeader, Color32, Grid, Layout, RichText, ScrollArea, Vec2, WidgetText,
};

use crate::animations::Frame;
use crate::{animation_window, animations, hats, prelude::*};

use eframe::egui::{DragValue, Ui};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex, TabViewer};
use num_traits::ToPrimitive;

use crate::animation_window::{AnimationWindow, AnimationWindowFrameData};
use crate::event_bus::EventBus;
use crate::frames_from_range::frames_from_range;
use crate::hats::{AbstractHat, Hat, HatType, LinkFrameState};
use crate::hats::{Extra, FlyingPet, WalkingPet, Wereable, Wings};
use crate::renderer::Renderer;
use crate::{egui_utils, FrameData};
pub enum NewHatEvent {
    Opened(std::path::PathBuf),
    New,
}

pub static HAT_EVENT_BUS: Mutex<EventBus<NewHatEvent>> = Mutex::new(EventBus::new());

pub struct Tabs {
    pub dock_state: DockState<Tab>,
    pub hat_tabs_counter: usize,
    pub home_tabs_counter: usize,
}

impl Tabs {
    pub fn new(home_name: String) -> Self {
        let mut dock_state = DockState::new(vec![Tab::new_home(home_name)]);
        dock_state.set_focused_node_and_surface((SurfaceIndex(0), NodeIndex(0)));
        Self {
            dock_state,
            hat_tabs_counter: 1,
            home_tabs_counter: 2,
        }
    }
    pub fn ui(&mut self, ui: &mut Ui, frame_data: crate::FrameData) {
        let mut added_nodes = vec![];
        DockArea::new(&mut self.dock_state)
            .show_add_buttons(true)
            .show_inside(
                ui,
                &mut MyTabViewer {
                    added_nodes: &mut added_nodes,
                    frame_data,
                },
            );
        for (surface, node) in added_nodes {
            let tab = Tab::new(format!("Hat {0}", self.hat_tabs_counter), Hat::default());
            self.dock_state
                .set_focused_node_and_surface((surface, node));
            self.dock_state.push_to_focused_leaf(tab);
            self.hat_tabs_counter += 1;
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum SelectedHat {
    Wereable,
    Extra,
    Wings,
    Room,
    Preview,
    Pet(usize),
}

impl SelectedHat {
    pub fn from_hat_type(value: HatType, pets: Option<&[Box<dyn AbstractHat>]>) -> Option<Self> {
        match value {
            HatType::Wereable => Some(Self::Wereable),
            HatType::Wings => Some(Self::Wings),
            HatType::Extra => Some(Self::Extra),
            HatType::Room => Some(Self::Room),
            HatType::Preview => Some(Self::Preview),
            HatType::Unspecified => unreachable!(),
            _ => {
                let pets = pets?;
                for (i, pet) in pets.iter().enumerate() {
                    if pet.base().hat_type == value {
                        return Some(Self::Pet(i));
                    }
                }
                None
            }
        }
    }
}

#[derive(Debug)]
pub struct TabInner {
    pub title: String,
    pub hat: Hat,
    pub is_home_tab: bool,
    pub selected_hat_type: Option<SelectedHat>,
    pub renderer: Option<Renderer>,
    pub animation_window: AnimationWindow,
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
            is_home_tab: false,
            selected_hat_type: None,
            renderer: None,
            animation_window: AnimationWindow::new(),
        });
        Self { inner }
    }

    pub fn new_home(title: String) -> Self {
        let inner = RefCell::new(TabInner {
            title,
            hat: Hat::default(),
            is_home_tab: true,
            selected_hat_type: None,
            renderer: None,
            animation_window: AnimationWindow::new(),
        });
        Self { inner }
    }
}
#[derive(Default)]
struct AnimationChanges {
    added: Option<AnimationType>,
    removed: Option<AnimationType>,
}

impl AnimationChanges {
    fn new(added: Option<AnimationType>, removed: Option<AnimationType>) -> Self {
        Self { added, removed }
    }
}

pub struct MyTabViewer<'a> {
    added_nodes: &'a mut Vec<(SurfaceIndex, NodeIndex)>,
    frame_data: FrameData<'a>,
}

impl MyTabViewer<'_> {
    fn draw_extra_hat_ui(&mut self, ui: &mut Ui, hat: &mut Extra) {}
    fn draw_wings_ui(&mut self, ui: &mut Ui, hat: &mut Wings) {}
    fn draw_flying_pet_ui(&mut self, ui: &mut Ui, hat: &mut FlyingPet) {}
    fn draw_walking_pet_ui(&mut self, ui: &mut Ui, hat: &mut WalkingPet) {}
    fn draw_wereable_hat_ui(&mut self, ui: &mut Ui, hat: &mut Wereable) {
        ScrollArea::new([true, true])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                ui.allocate_space((ui.available_width(), 1.0).into());
                ui.heading("Wereable hat")
                    .on_hover_text("This a wereable hat.\nIt can do stuff.");
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(
                        DragValue::new(&mut hat.base.frame_size.x)
                            .speed(0.2)
                            .clamp_range(hats::MIN_FRAME_SIZE..=hats::MAX_FRAME_SIZE),
                    );
                    ui.label("Y:");
                    ui.add(DragValue::new(&mut hat.base.frame_size.y).clamp_range(32..=64));
                    ui.label("Frame size");
                });
                egui::ComboBox::from_label("Quack Frame Link State")
                    .selected_text(format!("{}", hat.link_frame_state))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut hat.link_frame_state,
                            LinkFrameState::Default,
                            "None",
                        );
                        ui.selectable_value(
                            &mut hat.link_frame_state,
                            LinkFrameState::Saved,
                            "Saved",
                        );
                        ui.selectable_value(
                            &mut hat.link_frame_state,
                            LinkFrameState::Inverted,
                            "Inverted",
                        );
                    });
                let anim_changes = self.draw_animations_ui(hat as &mut dyn AbstractHat, ui);
                if let Some(anim) = anim_changes.added {
                    hat.animations.push(Animation::new(anim, 3, false, vec![]));
                }
                if let Some(anim) = anim_changes.removed {
                    hat.animations.retain(|a| a.anim_type == anim);
                }
            });
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
        CollapsingHeader::new("Add an animation")
            .enabled(can_add_animations)
            .show(ui, |ui| {
                for anim in avalible_anims {
                    if ui.button(anim.to_string()).clicked() {
                        anim_to_add = Some(anim);
                    }
                }
            });
        for anim in hat.animations_mut().unwrap_or_default() {
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
                        ui.push_id(item.id().0, |ui| {
                            ui.horizontal(|ui| {
                                handle.ui(ui, |ui| {
                                    ui.label(item.value.to_string());
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
                    anim.frames.insert(index, anim.frames[index].clone());
                }

                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut anim.new_frame)).changed();
                    if ui.button("Add Frame").clicked()
                        && (0..frames_amount).contains(&anim.new_frame.to_u32().unwrap_or(0))
                    {
                        anim.frames.push(anim.new_frame.into());
                        anim.new_frame += 1;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Start:");
                    ui.add(
                        egui::DragValue::new(&mut anim.new_range_start).clamp_range(0..=i32::MAX),
                    )
                    .changed();
                    ui.label("End:");
                    ui.add(egui::DragValue::new(&mut anim.new_range_end).clamp_range(0..=i32::MAX))
                        .changed();
                    ui.label(" ");
                    if ui.button("Set Frame Range").clicked() {
                        anim.frames = frames_from_range(anim.new_range_start, anim.new_range_end);
                    }
                });
                if ui.button("Clear Frames").clicked() {
                    anim.frames.clear();
                }
                if egui_utils::red_button(ui, "Delete", self.frame_data.config.is_light_theme())
                    .clicked()
                {
                    anim_to_delete = Some(anim.anim_type);
                }
            });
        }
        AnimationChanges::new(anim_to_add.copied(), anim_to_delete)
    }

    fn ui_home(&mut self, ui: &mut Ui) -> Option<NewHatEvent> {
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
        let mut new_hat_event = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(text.get("To start"));
            if ui.link(text.get("Open")).clicked() {
                if let Some(dir_path) = rfd::FileDialog::new().pick_folder() {
                    new_hat_event = Some(NewHatEvent::Opened(dir_path));
                }
            }
            ui.label(text.get("or create"));
            if ui.link(text.get("New")).clicked() {
                new_hat_event = Some(NewHatEvent::New);
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
        new_hat_event
    }
}
impl TabViewer for MyTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.inner.borrow().title.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let mut inner = tab.inner.borrow_mut();
        if inner.is_home_tab {
            let new_hat = self.ui_home(ui);
            if let Some(hat) = new_hat {
                (*HAT_EVENT_BUS.lock().unwrap()).send(hat);
            }
            return;
        }
        if inner.selected_hat_type.is_none() && !inner.hat.has_elements() {
            ui.label("Looks like this has is totaly empty! Maybe add an element or two?");
            return;
        }
        let selected_hat_type = inner.selected_hat_type.unwrap_or(
            SelectedHat::from_hat_type(
                inner
                    .hat
                    .iter_all_elements()
                    .next()
                    .unwrap()
                    .base()
                    .hat_type,
                Some(&inner.hat.pets),
            )
            .unwrap(),
        );
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
        match selected_hat_type {
            //TODO: pets should also get an animation window
            selected_type if !matches!(selected_type, SelectedHat::Pet(_)) => {
                //keep calm and call clone, right?
                if let Some((animations, texture, frame_size)) = inner
                    .hat
                    .unique_elemets
                    .get_mut(&HatType::try_from(selected_type).unwrap())
                    .and_then(|h| try { (h.animations()?, h.texture()?, h.base().frame_size) })
                    .map(|t| (t.0.to_vec(), t.1.clone(), t.2))
                {
                    let hat_name = inner.title.clone();
                    inner.animation_window.draw(AnimationWindowFrameData {
                        ui,
                        shader: self.frame_data.shader.clone(),
                        hertz: self.frame_data.hertz as f32,
                        animations,
                        texture,
                        frame_size,
                        hat_name,
                        anim_window_action: self.frame_data.anim_window_action,
                    });
                }
            }
            _ => {}
        };
        match selected_hat_type {
            SelectedHat::Wereable => {
                if let Some(hat) = &mut inner.hat.wereable_mut() {
                    self.draw_wereable_hat_ui(ui, hat);
                }
            }
            SelectedHat::Extra => {
                if let Some(hat) = &mut inner.hat.extra_mut() {
                    self.draw_extra_hat_ui(ui, hat);
                }
            }
            _ => (),
        }
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
