//! Selection result delivery.
//!
//! Owns the intent → `OverlaySelection` mapping used when the user confirms
//! a capture or OCR selection. Callers must release the state mutex
//! before GTK window closure (this function locks briefly then closes).

use super::super::api::{OverlaySelection, SelectionResult};
use super::super::background::BackgroundFrame;
use super::super::geometry::selection_area_from_state;
use super::super::state::{OverlayIntent, SelectorState};
use gtk4::prelude::*;
use gtk4::ApplicationWindow;
use std::sync::{Arc, Mutex};

/// Map the active intent + selection to an `OverlaySelection`, send it on
/// `result_tx`, and close the overlay window.
///
/// Preserves:
/// - screenshot/background coordinate mapping via `selection_area_from_state`
/// - OCR and area both deliver `Area(Some)` when valid
/// - invalid selection → `Area(None)` for every intent
pub(crate) fn send_selection_result(
    state: &Arc<Mutex<SelectorState>>,
    result_tx: &std::sync::mpsc::Sender<SelectionResult>,
    window: &ApplicationWindow,
    screen_width: i32,
    screen_height: i32,
    background: Option<&BackgroundFrame>,
) {
    let st = state.lock().unwrap();
    let area = selection_area_from_state(&st, screen_width, screen_height, background);
    let intent = st.intent;
    let result = match intent {
        OverlayIntent::Ocr => {
            if area.is_valid() {
                Ok(OverlaySelection::Area(Some(area)))
            } else {
                Ok(OverlaySelection::Area(None))
            }
        }
        OverlayIntent::Area => {
            if area.is_valid() {
                Ok(OverlaySelection::Area(Some(area)))
            } else {
                Ok(OverlaySelection::Area(None))
            }
        }
    };
    drop(st);
    let _ = result_tx.send(result);
    window.close();
}

#[cfg(test)]
mod tests {
    /// Owner contract: result delivery lives here.
    #[test]
    fn result_owner_covers_delivery() {
        let source = include_str!("result.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production result source");
        assert!(
            production.contains("fn send_selection_result"),
            "result must own send_selection_result"
        );
        assert!(
            production.contains("OverlayIntent::Ocr") && production.contains("OverlayIntent::Area"),
            "result must map the remaining overlay intents"
        );
        assert!(
            !production.contains("OverlayIntent::Record")
                && !production.contains("RecordingRequest"),
            "the retired recording request path must stay removed"
        );
        assert!(
            production.contains("OverlaySelection::Area(None)"),
            "invalid selection must still yield Area(None)"
        );
        assert!(
            production.contains("selection_area_from_state"),
            "result must keep screenshot/background coordinate mapping"
        );
        assert!(
            production.contains("window.close()"),
            "result delivery closes the overlay window"
        );
    }
}
