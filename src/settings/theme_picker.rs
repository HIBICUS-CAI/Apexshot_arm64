//! Light / Dark / System theme picker for Settings → General.
//!
//! Each option renders a miniature window mock so the choice reads as a
//! preview instead of an abstract label. The selection ring lives on the
//! thumbnail wrapper (`settings-theme-ring`) so the caption stays outside it,
//! matching the Appearance design.

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::config::DEFAULT_UI_THEME;
use crate::i18n::t;

/// Stored ids in display order. Captions come from [`theme_caption`] so the
/// strings stay literal for the catalog coverage checker.
const THEME_IDS: [&str; 3] = ["light", "dark", "system"];

fn theme_caption(theme_id: &str) -> String {
    match theme_id {
        "light" => t("Light"),
        "dark" => t("Dark"),
        _ => t("System"),
    }
}

#[derive(Clone)]
pub struct ThemePicker {
    root: GtkBox,
    ids: Rc<Vec<String>>,
    selected: Rc<Cell<usize>>,
    on_changed: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl ThemePicker {
    pub fn new(current: &str) -> Self {
        let ids: Vec<String> = THEME_IDS.iter().map(|id| (*id).to_string()).collect();
        let selected_index = ids
            .iter()
            .position(|id| id == current)
            .or_else(|| ids.iter().position(|id| id == DEFAULT_UI_THEME))
            .unwrap_or(0);
        let ids = Rc::new(ids);
        let selected = Rc::new(Cell::new(selected_index));
        let on_changed: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        let root = GtkBox::new(Orientation::Horizontal, 6);
        root.add_css_class("settings-theme-picker");
        root.set_halign(Align::End);
        root.set_valign(Align::Center);

        let mut options = Vec::with_capacity(THEME_IDS.len());
        for (index, id) in THEME_IDS.iter().enumerate() {
            let option = build_option(id);
            if index == selected_index {
                option.add_css_class("settings-theme-option-selected");
            }
            root.append(&option);
            options.push(option);
        }
        let options = Rc::new(options);

        for (index, option) in options.iter().enumerate() {
            let options = Rc::clone(&options);
            let selected = Rc::clone(&selected);
            let on_changed = Rc::clone(&on_changed);
            option.connect_clicked(move |_| {
                if selected.get() == index {
                    return;
                }
                selected.set(index);
                for (i, option) in options.iter().enumerate() {
                    if i == index {
                        option.add_css_class("settings-theme-option-selected");
                    } else {
                        option.remove_css_class("settings-theme-option-selected");
                    }
                }
                for callback in on_changed.borrow().iter() {
                    callback();
                }
            });
        }

        Self {
            root,
            ids,
            selected,
            on_changed,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.root
    }

    pub fn active_id(&self) -> Option<String> {
        self.ids.get(self.selected.get()).cloned()
    }

    pub fn connect_changed<F: Fn() + 'static>(&self, callback: F) {
        self.on_changed.borrow_mut().push(Rc::new(callback));
    }
}

fn build_option(theme_id: &str) -> Button {
    let caption = theme_caption(theme_id);
    let button = Button::new();
    button.add_css_class("settings-theme-option");
    button.set_has_frame(false);
    button.set_focus_on_click(false);
    button.set_tooltip_text(Some(&caption));

    let column = GtkBox::new(Orientation::Vertical, 4);
    column.set_halign(Align::Center);

    let ring = GtkBox::new(Orientation::Vertical, 0);
    ring.add_css_class("settings-theme-ring");
    ring.set_halign(Align::Center);
    ring.append(&build_preview(theme_id));

    let label = Label::new(Some(&caption));
    label.add_css_class("settings-theme-option-label");

    column.append(&ring);
    column.append(&label);
    button.set_child(Some(&column));
    button
}

/// Miniature app window: title-bar dots plus text lines, laid over a
/// theme-tinted card. Purely decorative — the whole option is the control.
fn build_preview(theme_id: &str) -> GtkBox {
    let card = GtkBox::new(Orientation::Vertical, 0);
    card.add_css_class("settings-theme-preview");
    card.add_css_class(&format!("settings-theme-preview-{theme_id}"));
    card.set_size_request(84, 52);
    card.set_halign(Align::Center);
    card.set_valign(Align::Center);
    card.set_overflow(gtk4::Overflow::Hidden);

    let window = GtkBox::new(Orientation::Vertical, 3);
    window.add_css_class("settings-theme-preview-window");
    window.set_halign(Align::Center);
    window.set_valign(Align::Center);

    let titlebar = GtkBox::new(Orientation::Horizontal, 3);
    titlebar.add_css_class("settings-theme-preview-titlebar");
    for _ in 0..2 {
        let dot = GtkBox::new(Orientation::Horizontal, 0);
        dot.add_css_class("settings-theme-preview-dot");
        dot.set_size_request(4, 4);
        titlebar.append(&dot);
    }
    window.append(&titlebar);

    for width in [24, 30, 16] {
        let line = GtkBox::new(Orientation::Horizontal, 0);
        line.add_css_class("settings-theme-preview-line");
        line.set_size_request(width, 3);
        line.set_halign(Align::Start);
        window.append(&line);
    }

    card.append(&window);
    card
}
