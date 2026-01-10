use super::super::EntryKind;
use super::super::{Bus, Event, Hub, Id, RenderData, RenderQueue, View, ID_FEEDER};
use crate::color::{TEXT_INVERTED_HARD, TEXT_NORMAL};
use crate::context::Context;
use crate::device::CURRENT_DEVICE;
use crate::font::{font_from_style, Fonts, NORMAL_STYLE};
use crate::framebuffer::{Framebuffer, UpdateMode};
use crate::geom::Rectangle;
use crate::gesture::GestureEvent;
use crate::input::{DeviceEvent, FingerStatus};
use crate::view::menu::Menu;
use crate::view::EntryId;

pub struct SettingValue {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    value: String,
    active: bool,
    entries: Vec<EntryKind>,
}

impl SettingValue {
    pub fn new(rect: Rectangle, value: String, entries: Vec<EntryKind>) -> SettingValue {
        SettingValue {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            value,
            active: false,
            entries,
        }
    }

    pub fn update(&mut self, value: String, rq: &mut RenderQueue) {
        if self.value != value {
            self.value = value;
            rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
        }
    }
}

impl View for SettingValue {
    fn handle_event(
        &mut self,
        evt: &Event,
        _hub: &Hub,
        bus: &mut Bus,
        rq: &mut RenderQueue,
        context: &mut Context,
    ) -> bool {
        match *evt {
            Event::Device(DeviceEvent::Finger {
                status, position, ..
            }) => match status {
                FingerStatus::Down if self.rect.includes(position) => {
                    self.active = true;

                    rq.add(RenderData::new(self.id, self.rect, UpdateMode::Fast));

                    true
                }
                FingerStatus::Up if self.active => {
                    self.active = false;

                    rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));

                    true
                }
                _ => false,
            },
            Event::Gesture(GestureEvent::Tap(point)) if self.rect.includes(point) => {
                let menu = Menu::new(
                    self.rect,
                    crate::view::ViewId::SettingsKeyboardLayoutMenu,
                    crate::view::menu::MenuKind::SubMenu,
                    self.entries.clone(),
                    context,
                )
                .root(true);

                rq.add(RenderData::new(menu.id(), *menu.rect(), UpdateMode::Gui));
                self.children.push(Box::new(menu));

                true
            }
            Event::Select(EntryId::SetKeyboardLayout(ref selected_layout)) => {
                for entry in &mut self.entries {
                    if let EntryKind::RadioButton(
                        _,
                        EntryId::SetKeyboardLayout(ref layout),
                        ref mut selected,
                    ) = entry
                    {
                        *selected = layout == selected_layout
                    }
                }

                self.update(selected_layout.clone(), rq);

                false
            }
            Event::Validate => {
                self.children.clear();

                bus.push_back(Event::Close(
                    crate::view::ViewId::SettingsKeyboardLayoutMenu,
                ));

                true
            }
            Event::Close(view_id) if view_id == crate::view::ViewId::SettingsKeyboardLayoutMenu => {
                self.children.clear();

                false
            }
            _ => false,
        }
    }

    fn render(&self, fb: &mut dyn Framebuffer, rect: Rectangle, fonts: &mut Fonts) {
        let dpi = CURRENT_DEVICE.dpi;
        let font = font_from_style(fonts, &NORMAL_STYLE, dpi);
        let x_height = font.x_heights.0 as i32;
        let padding = font.em() as i32;

        let scheme = if self.active {
            TEXT_INVERTED_HARD
        } else {
            TEXT_NORMAL
        };

        fb.draw_rectangle(&rect, scheme[0]);

        let max_width = rect.width() as i32 - padding;
        let plan = font.plan(&self.value, Some(max_width), None);
        let dy = (rect.height() as i32 - x_height) / 2;
        let dx = rect.width() as i32 - padding - plan.width;
        let pt = pt!(rect.min.x + dx, rect.max.y - dy);

        font.render(fb, scheme[1], &plan, pt);
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
