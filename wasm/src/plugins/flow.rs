//! The headings' gradient, set flowing.
//!
//! The stylesheet paints each heading with a gradient twice the heading's width
//! that runs from one colour to another and back, and places it by the custom
//! property `--flow` (0% to 100%). This plugin moves `--flow` once per frame
//! along an eased curve, so the colours drift back and forth across the
//! letters, slowing at each end rather than turning sharply. One property on the root
//! element moves every heading at once.
//!
//! The browser stops animation frames in a hidden tab, so the drift costs
//! nothing there. A reader who asks for reduced motion sees the gradient hold
//! still in its resting place, as it does without this module at all.

use crate::{dom, Plugin};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Document, HtmlElement};

pub const PLUGIN: Plugin = Plugin { name: "flow", install };

/// One full sweep there and back, in milliseconds.
const PERIOD_MS: f64 = 9000.0;
const SELECTOR: &str = ".title, .section-title";
const REDUCED_MOTION: &str = "(prefers-reduced-motion: reduce)";

fn install(document: &Document) -> Result<(), JsValue> {
    if dom::query(document, SELECTOR).is_none() {
        return Ok(());
    }
    let Some(root) = document.document_element().and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return Ok(());
    };
    let reduced = dom::media(REDUCED_MOTION);

    // The frame callback schedules itself again, so it must be able to refer
    // to itself: it lives in a shared slot that it also holds.
    let slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let next = slot.clone();
    let mut last = String::new();

    *slot.borrow_mut() = Some(Closure::wrap(Box::new(move |now: f64| {
        let still = reduced.as_ref().is_some_and(|list| list.matches());
        let position = if still { 0.0 } else { flow_at(now) };
        // Tenths of a percent, written with integer formatting: formatting a
        // float would pull in code that doubles the size of the module. Write
        // only when the value a reader could see has changed.
        let tenths = (position * 10.0 + 0.5) as u32;
        let value = format!("{}.{}%", tenths / 10, tenths % 10);
        if value != last {
            let _ = root.style().set_property("--flow", &value);
            last = value;
        }
        if let (Some(win), Some(callback)) = (window(), next.borrow().as_ref()) {
            let _ = win.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut(f64)>));

    if let (Some(win), Some(callback)) = (window(), slot.borrow().as_ref()) {
        win.request_animation_frame(callback.as_ref().unchecked_ref())?;
    }
    Ok(())
}

/// Where the gradient sits at `now` milliseconds: 0% at rest, easing out to
/// 100% and back over one period. A triangle wave smoothed by smoothstep: the
/// same gentle turn as a cosine, in plain arithmetic, which keeps the module
/// free of the maths library.
fn flow_at(now: f64) -> f64 {
    let phase = now / PERIOD_MS;
    let t = phase - (phase as u64) as f64; // 0..1 within the period
    let tri = 1.0 - (2.0 * t - 1.0).abs(); // 0 -> 1 -> 0
    100.0 * tri * tri * (3.0 - 2.0 * tri)
}

#[cfg(test)]
mod tests {
    use super::flow_at;

    #[test]
    fn rests_at_zero_and_reaches_the_far_end_halfway() {
        assert!(flow_at(0.0).abs() < 1e-9);
        assert!((flow_at(4500.0) - 100.0).abs() < 1e-9);
        assert!(flow_at(9000.0).abs() < 1e-9);
    }
}
