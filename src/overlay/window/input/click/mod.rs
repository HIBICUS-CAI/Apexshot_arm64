//! Primary overlay click wiring.

mod menu;
mod primary;
mod toolbar;

use crate::overlay::api::SelectionResult;
use crate::overlay::background::BackgroundFrame;
use crate::overlay::state::SelectorState;
use gtk4::{ApplicationWindow, DrawingArea};
use std::sync::{Arc, Mutex};

enum ClickEffect {
    None,
    Redraw,
    OpenScrollExtension,
    SendSelection,
    /// Top-bar X button: cancel the selection and close the overlay.
    Cancel,
}

pub(in crate::overlay::window) fn wire_window_click(
    window: &ApplicationWindow,
    state: Arc<Mutex<SelectorState>>,
    result_tx: std::sync::mpsc::Sender<SelectionResult>,
    drawing_area: &DrawingArea,
    background: Option<BackgroundFrame>,
    screen_width: i32,
    screen_height: i32,
) {
    primary::wire_primary_click(
        window,
        state,
        result_tx,
        drawing_area,
        background,
        screen_width,
        screen_height,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn click_facade_wires_primary() {
        let source = include_str!("mod.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production click facade");
        assert!(
            production.contains("primary::wire_primary_click("),
            "click facade must wire the primary click owner"
        );
        assert!(
            !production.contains("secondary::wire_secondary_click("),
            "the retired secondary (recording-panel) click must stay removed"
        );
        assert!(
            production.contains("mod menu;") && production.contains("mod toolbar;"),
            "click facade must declare cohesive menu and toolbar owners"
        );
        assert!(
            !production.contains("SendRecording")
                && !production.contains("SetMicVolume")
                && !production.contains("SetSpeakerVolume"),
            "the retired recording-panel click effects must stay removed"
        );
    }
}
