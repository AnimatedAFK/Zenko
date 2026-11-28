//! The koi book's own window (zenko.py Zenko22.koi_book's Toplevel): opened once (again it only comes to the front),
//! its tiles picked with a click, the grid scrolled with the wheel, the pond toggle saved, Esc closes it.

use std::cell::RefCell;
use zenko_desktop::gardenpage::GardenData;
use zenko_desktop::koibook::{KoiBook, TITLE, geometry};
use zenko_desktop::theme::Theme;
use zenko_desktop::ui::Ui;

const NAME: &str = "koibook";

thread_local! {
    static KOI: RefCell<Option<KoiBook>> = const { RefCell::new(None) };
}

pub fn open(d: &GardenData, th: &Theme, font_ui: &str, dpi: f64) {
    if crate::uiwin::is_open(NAME) {
        crate::uiwin::raise(NAME);
        return;
    }
    let (sw, sh) = unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
    };
    let (w, h, x, y) = geometry(sw, sh);
    let mut ui = Ui::new(&th.paper);
    let cm10 = (100.0 * dpi / 25.4 + 0.5) as i32;
    let jp = if d.jp.is_empty() { font_ui.to_string() } else { d.jp.clone() };
    let k = KoiBook::build(&mut ui, th, font_ui, &jp, &d.serif, &d.fishing, cm10);
    KOI.with(|s| *s.borrow_mut() = Some(k));
    crate::uiwin::open_window(NAME, TITLE, ui, th.clone(), dpi, x, y, w, h);
    fit(w, h);
}

fn fit(w: i32, h: i32) {
    KOI.with(|s| {
        if let Some(k) = s.borrow_mut().as_mut() {
            crate::uiwin::with_ui(NAME, |ui, fonts| k.fit(ui, fonts, w, h));
        }
    });
}

fn size() -> (i32, i32) {
    crate::uiwin::with_ui(NAME, |ui, _| (ui.ws[0].w, ui.ws[0].h)).unwrap_or((980, 680))
}

/// what the window sent: true when it was the koi book's
pub fn out(o: &crate::uiwin::UiOut) -> bool {
    if o.win != NAME {
        return false;
    }
    let a: Vec<&str> = o.cmd.split(':').collect();
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    match a[0] {
        "resize" => fit(num(1), num(2)),
        "wheel" => KOI.with(|s| {
            if let Some(k) = s.borrow_mut().as_mut() {
                crate::uiwin::with_ui(NAME, |ui, _| k.wheel(ui, num(1)));
            }
        }),
        "click" => {
            let hit = KOI.with(|s| {
                s.borrow_mut().as_mut().and_then(|k| crate::uiwin::with_ui(NAME, |ui, _| k.click(ui, num(1), num(2))))
            }).unwrap_or(false);
            if hit {
                let (w, h) = size();
                fit(w, h);
            }
        }
        "koi_pond" => {
            crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Ui("tap".into())));
            let pond = KOI.with(|s| s.borrow_mut().as_mut().and_then(|k| k.toggle_pond()));
            if let Some(p) = pond {
                crate::win::with_fox(move |fox, _| fox.life_do(|l| l["fishing"]["pond"] = p));
                crate::hubw::garden_pond_changed();
                KOI.with(|s| {
                    if let Some(k) = s.borrow_mut().as_mut() {
                        crate::uiwin::with_ui(NAME, |ui, _| k.detail(ui));
                    }
                });
                let (w, h) = size();
                fit(w, h);
            }
        }
        "key" if num(1) == 0x1b => {
            crate::uiwin::close(NAME);
            KOI.with(|s| *s.borrow_mut() = None);
        }
        "closed" => KOI.with(|s| *s.borrow_mut() = None),
        _ => {}
    }
    true
}
