use crate::color::{BLACK, WHITE};
use crate::context::Context;
use crate::device::CURRENT_DEVICE;
use crate::framebuffer::{Framebuffer, UpdateMode};
use crate::geom::{halves, Rectangle};
use crate::helpers::save_toml;
use crate::settings::SETTINGS_PATH;
use crate::unit::scale_by_dpi;
use crate::view::filler::Filler;
use crate::view::icon::Icon;

use crate::view::top_bar::TopBar;
use crate::view::{Bus, Event, Hub, Id, RenderData, RenderQueue, View, ID_FEEDER};
use crate::view::{EntryId, EntryKind};
use crate::view::{BIG_BAR_HEIGHT, SMALL_BAR_HEIGHT, THICKNESS_MEDIUM};
use anyhow::Error;
use std::fs;
use std::path::Path;
mod setting_row;
mod setting_value;
pub use self::setting_row::SettingRow;
pub use self::setting_value::SettingValue;

pub struct SettingsEditor {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    original_keyboard_layout: String,
}

impl SettingsEditor {
    pub fn new(
        rect: Rectangle,
        _hub: &Hub,
        rq: &mut RenderQueue,
        context: &mut Context,
    ) -> Result<SettingsEditor, Error> {
        let id = ID_FEEDER.next();
        let mut children = Vec::new();
        let dpi = CURRENT_DEVICE.dpi;
        let (small_height, _big_height) = (
            scale_by_dpi(SMALL_BAR_HEIGHT, dpi) as i32,
            scale_by_dpi(BIG_BAR_HEIGHT, dpi) as i32,
        );
        let thickness = scale_by_dpi(THICKNESS_MEDIUM, dpi) as i32;
        let (small_thickness, big_thickness) = halves(thickness);
        let side = small_height;

        let original_keyboard_layout = context.settings.keyboard_layout.clone();

        let top_bar = TopBar::new(
            rect![
                rect.min.x,
                rect.min.y,
                rect.max.x,
                rect.min.y + side - small_thickness
            ],
            Event::Back,
            "Settings".to_string(),
            context,
        );
        children.push(Box::new(top_bar) as Box<dyn View>);

        let separator = Filler::new(
            rect![
                rect.min.x,
                rect.min.y + side - small_thickness,
                rect.max.x,
                rect.min.y + side + big_thickness
            ],
            BLACK,
        );
        children.push(Box::new(separator) as Box<dyn View>);

        let content_rect = rect![
            rect.min.x,
            rect.min.y + side + big_thickness,
            rect.max.x,
            rect.max.y - side - small_thickness
        ];

        let background = Filler::new(content_rect, WHITE);
        children.push(Box::new(background) as Box<dyn View>);

        let available_layouts = Self::get_available_layouts()?;
        let current_layout = context.settings.keyboard_layout.clone();

        let row_height = scale_by_dpi(BIG_BAR_HEIGHT, dpi) as i32;
        let row_rect = rect![
            content_rect.min.x,
            content_rect.min.y,
            content_rect.max.x,
            content_rect.min.y + row_height
        ];

        let entries: Vec<EntryKind> = available_layouts
            .iter()
            .map(|layout| {
                EntryKind::RadioButton(
                    layout.clone(),
                    EntryId::SetKeyboardLayout(layout.clone()),
                    current_layout == *layout,
                )
            })
            .collect();

        let setting_row = SettingRow::new(
            row_rect,
            "Keyboard Layout".to_string(),
            current_layout.clone(),
            entries,
        );
        children.push(Box::new(setting_row) as Box<dyn View>);

        let separator = Filler::new(
            rect![
                rect.min.x,
                rect.max.y - side - small_thickness,
                rect.max.x,
                rect.max.y - side + big_thickness
            ],
            BLACK,
        );
        children.push(Box::new(separator) as Box<dyn View>);

        let bottom_bar_rect = rect![
            rect.min.x,
            rect.max.y - side + big_thickness,
            rect.max.x,
            rect.max.y
        ];

        let button_width = bottom_bar_rect.width() as i32 / 2;

        let cancel_rect = rect![
            bottom_bar_rect.min.x,
            bottom_bar_rect.min.y,
            bottom_bar_rect.min.x + button_width,
            bottom_bar_rect.max.y
        ];

        let cancel_icon = Icon::new("back", cancel_rect, Event::Back);
        children.push(Box::new(cancel_icon) as Box<dyn View>);

        let save_rect = rect![
            bottom_bar_rect.min.x + button_width,
            bottom_bar_rect.min.y,
            bottom_bar_rect.max.x,
            bottom_bar_rect.max.y
        ];

        let save_icon = Icon::new("check_mark", save_rect, Event::Validate);
        children.push(Box::new(save_icon) as Box<dyn View>);

        rq.add(RenderData::new(id, rect, UpdateMode::Full));

        Ok(SettingsEditor {
            id,
            rect,
            children,
            original_keyboard_layout,
        })
    }

    fn get_available_layouts() -> Result<Vec<String>, Error> {
        let layouts_dir = Path::new("keyboard-layouts");
        let mut layouts = Vec::new();

        if layouts_dir.exists() {
            for entry in fs::read_dir(layouts_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let layout_name = stem
                            .chars()
                            .enumerate()
                            .map(|(i, c)| {
                                if i == 0 {
                                    c.to_uppercase().collect::<String>()
                                } else {
                                    c.to_string()
                                }
                            })
                            .collect::<String>();
                        layouts.push(layout_name);
                    }
                }
            }
        }

        layouts.sort();
        Ok(layouts)
    }
}

impl View for SettingsEditor {
    fn handle_event(
        &mut self,
        evt: &Event,
        hub: &Hub,
        _bus: &mut Bus,
        rq: &mut RenderQueue,
        context: &mut Context,
    ) -> bool {
        match *evt {
            Event::Select(EntryId::SetKeyboardLayout(ref layout)) => {
                context.settings.keyboard_layout = layout.clone();

                true
            }
            Event::Validate => {
                hub.send(Event::Back).ok();

                if let Err(e) = save_toml(&context.settings, SETTINGS_PATH) {
                    eprintln!("Failed to save settings: {:#}", e);
                    hub.send(Event::Notify("Failed to save settings".to_string()))
                        .ok();
                } else {
                    hub.send(Event::Notify("Settings saved successfully".to_string()))
                        .ok();
                }

                true
            }
            Event::Back => {
                context.settings.keyboard_layout = self.original_keyboard_layout.clone();

                false
            }

            Event::Close(..) => {
                rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));

                true
            }
            _ => false,
        }
    }

    fn render(&self, _fb: &mut dyn Framebuffer, _rect: Rectangle, _fonts: &mut crate::font::Fonts) {
    }

    fn is_background(&self) -> bool {
        true
    }

    fn rect(&self) -> &Rectangle {
        &self.rect
    }

    fn rect_mut(&mut self) -> &mut Rectangle {
        &mut self.rect
    }

    fn children(&self) -> &Vec<Box<dyn View>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> {
        &mut self.children
    }

    fn id(&self) -> Id {
        self.id
    }
}
