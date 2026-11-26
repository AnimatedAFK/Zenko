//! The right-click menu (zenko.py on_menu + menu_extras (ZenkoExtras, ZenkoLife) + style_menus). Tk draws its menus
//! on Windows as owner-drawn native menus in the theme colours (FIELD background, INK text, ORANGE/ON_ACC when
//! highlighted, GREY when disabled); this does the same.

use crate::gdi::{self, rgb};
use std::collections::HashMap;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_desktop::theme::Theme;

pub enum Item {
    Cmd { label: String, cmd: String, enabled: bool },
    Check { label: String, cmd: String, on: bool, enabled: bool },
    Radio { label: String, cmd: String, on: bool },
    Sep,
    Sub { label: String, items: Vec<Item> },
}

pub fn cmd(label: impl Into<String>, c: &str) -> Item {
    Item::Cmd { label: label.into(), cmd: c.into(), enabled: true }
}
/// A command whose feature is not in the Rust port yet: shown, greyed out.
pub fn later(label: impl Into<String>) -> Item {
    Item::Cmd { label: label.into(), cmd: String::new(), enabled: false }
}
pub fn info(label: impl Into<String>) -> Item {
    Item::Cmd { label: label.into(), cmd: String::new(), enabled: false }
}
pub fn check(label: impl Into<String>, c: &str, on: bool) -> Item {
    Item::Check { label: label.into(), cmd: c.into(), on, enabled: true }
}
pub fn radio(label: impl Into<String>, c: &str, on: bool) -> Item {
    Item::Radio { label: label.into(), cmd: c.into(), on }
}
pub fn sub(label: impl Into<String>, items: Vec<Item>) -> Item {
    Item::Sub { label: label.into(), items }
}

/// What owner-draw needs for one item.
pub struct Drawn {
    pub label: Vec<u16>,
    pub sep: bool,
    pub mark: u8, // 0 none, 1 check, 2 radio
    pub enabled: bool,
}

pub struct MenuCtx {
    pub drawn: HashMap<usize, Drawn>,
    pub cmds: HashMap<u32, String>,
    pub theme: Theme,
    pub font: HFONT,
    next_id: u32,
}

impl MenuCtx {
    fn build(&mut self, items: &[Item]) -> HMENU {
        unsafe {
            let m = CreatePopupMenu();
            for it in items {
                let key = self.drawn.len() + 1;
                let (label, sep, mark, enabled) = match it {
                    Item::Cmd { label, enabled, .. } => (label.clone(), false, 0, *enabled),
                    Item::Check { label, on, enabled, .. } => (label.clone(), false, if *on { 1 } else { 0 }, *enabled),
                    Item::Radio { label, on, .. } => (label.clone(), false, if *on { 2 } else { 0 }, true),
                    Item::Sep => (String::new(), true, 0, true),
                    Item::Sub { label, .. } => (label.clone(), false, 0, true),
                };
                self.drawn.insert(key, Drawn { label: label.encode_utf16().collect(), sep, mark, enabled });
                let mut flags = MF_OWNERDRAW;
                if !enabled {
                    flags |= MF_GRAYED;
                }
                match it {
                    Item::Sub { items, .. } => {
                        let s = self.build(items);
                        AppendMenuW(m, flags | MF_POPUP, s as usize, key as *const u16);
                    }
                    Item::Sep => {
                        AppendMenuW(m, MF_OWNERDRAW | MF_SEPARATOR, 0, key as *const u16);
                    }
                    Item::Cmd { cmd, .. } | Item::Check { cmd, .. } | Item::Radio { cmd, .. } => {
                        self.next_id += 1;
                        self.cmds.insert(self.next_id, cmd.clone());
                        AppendMenuW(m, flags, self.next_id as usize, key as *const u16);
                    }
                }
                // owner-drawn items carry the key in itemData
                let mut mii: MENUITEMINFOW = std::mem::zeroed();
                mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
                mii.fMask = MIIM_DATA;
                mii.dwItemData = key;
                SetMenuItemInfoW(m, (GetMenuItemCount(m) - 1) as u32, 1, &mii);
            }
            m
        }
    }
}

thread_local! {
    pub static CTX: std::cell::RefCell<Option<MenuCtx>> = const { std::cell::RefCell::new(None) };
}

fn menu_font() -> HFONT {
    unsafe {
        let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, ncm.cbSize, &mut ncm as *mut _ as *mut _, 0);
        CreateFontIndirectW(&ncm.lfMenuFont)
    }
}

/// Show the menu at the mouse and return the chosen command ("" = nothing).
pub fn popup(owner: HWND, items: &[Item], theme: &Theme) -> String {
    let font = menu_font();
    let m = CTX.with(|c| {
        let mut ctx = MenuCtx { drawn: HashMap::new(), cmds: HashMap::new(), theme: theme.clone(), font, next_id: 0 };
        let m = ctx.build(items);
        *c.borrow_mut() = Some(ctx);
        m
    });
    let chosen = unsafe {
        let bg = CreateSolidBrush(rgb(&theme.field));
        let mut mi: MENUINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MENUINFO>() as u32;
        mi.fMask = MIM_BACKGROUND | MIM_APPLYTOSUBMENUS;
        mi.hbrBack = bg;
        SetMenuInfo(m, &mi);
        let mut p = POINT { x: 0, y: 0 };
        GetCursorPos(&mut p);
        SetForegroundWindow(owner);
        let id = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, p.x, p.y, 0, owner, std::ptr::null());
        PostMessageW(owner, WM_NULL, 0, 0);
        DestroyMenu(m);
        DeleteObject(bg as _);
        id as u32
    };
    let out = CTX.with(|c| c.borrow_mut().take().and_then(|ctx| ctx.cmds.get(&chosen).cloned()).unwrap_or_default());
    unsafe { DeleteObject(font as _) };
    out
}

fn text_size(dc: HDC, s: &[u16]) -> SIZE {
    let mut sz = SIZE { cx: 0, cy: 0 };
    unsafe { GetTextExtentPoint32W(dc, s.as_ptr(), s.len() as i32, &mut sz) };
    sz
}

/// WM_MEASUREITEM for the owner-drawn items.
pub unsafe fn measure(mis: *mut MEASUREITEMSTRUCT) {
    unsafe {
        let mis = &mut *mis;
        CTX.with(|c| {
            let g = c.borrow();
            let Some(ctx) = g.as_ref() else { return };
            let Some(d) = ctx.drawn.get(&mis.itemData) else { return };
            let dc = GetDC(std::ptr::null_mut());
            let of = SelectObject(dc, ctx.font as _);
            let mut tm: TEXTMETRICW = std::mem::zeroed();
            GetTextMetricsW(dc, &mut tm);
            if d.sep {
                mis.itemHeight = 7;
                mis.itemWidth = 10;
            } else {
                let sz = text_size(dc, &d.label);
                let ind = GetSystemMetrics(SM_CXMENUCHECK);
                mis.itemWidth = (ind + 6 + sz.cx + 24) as u32;
                mis.itemHeight = (tm.tmHeight + 6) as u32;
            }
            SelectObject(dc, of);
            ReleaseDC(std::ptr::null_mut(), dc);
        });
    }
}

/// WM_DRAWITEM for the owner-drawn items.
pub unsafe fn draw(dis: *const DRAWITEMSTRUCT) {
    unsafe {
        let dis = &*dis;
        CTX.with(|c| {
            let g = c.borrow();
            let Some(ctx) = g.as_ref() else { return };
            let Some(d) = ctx.drawn.get(&dis.itemData) else { return };
            let t = &ctx.theme;
            let dc = dis.hDC;
            let rc = dis.rcItem;
            let active = dis.itemState & ODS_SELECTED != 0 && d.enabled && !d.sep;
            let bg = CreateSolidBrush(rgb(if active { &t.acc } else { &t.field }));
            FillRect(dc, &rc, bg);
            DeleteObject(bg as _);
            if d.sep {
                let mut r = rc;
                r.top += 3;
                r.bottom = r.top + 2;
                DrawEdge(dc, &mut r, EDGE_ETCHED, BF_TOP);
                return;
            }
            let fg = if !d.enabled {
                rgb(&t.grey)
            } else if active {
                rgb(&t.onacc)
            } else {
                rgb(&t.ink)
            };
            let of = SelectObject(dc, ctx.font as _);
            SetBkMode(dc, TRANSPARENT as i32);
            SetTextColor(dc, fg);
            let ind = GetSystemMetrics(SM_CXMENUCHECK);
            let sz = text_size(dc, &d.label);
            let ty = rc.top + (rc.bottom - rc.top - sz.cy) / 2;
            TextOutW(dc, rc.left + ind + 6, ty, d.label.as_ptr(), d.label.len() as i32);
            if d.mark != 0 {
                // the check / radio mark in the select colour (Tk: selectcolor = GOLD)
                let (w, h) = (ind, GetSystemMetrics(SM_CYMENUCHECK));
                let mdc = CreateCompatibleDC(dc);
                let mono = CreateBitmap(w, h, 1, 1, std::ptr::null());
                let ob = SelectObject(mdc, mono as _);
                let mut r = RECT { left: 0, top: 0, right: w, bottom: h };
                DrawFrameControl(mdc, &mut r, DFC_MENU, if d.mark == 1 { DFCS_MENUCHECK } else { DFCS_MENUBULLET });
                let y = rc.top + (rc.bottom - rc.top - h) / 2;
                let brush = CreateSolidBrush(rgb(if active { &t.onacc } else { &t.gold }));
                let obr = SelectObject(dc, brush as _);
                SetTextColor(dc, 0x000000);
                SetBkColor(dc, 0xffffff);
                // where the mask is black, paint the brush (ROP: (mask AND brush) OR (NOT mask AND dest))
                BitBlt(dc, rc.left + 3, y, w, h, mdc, 0, 0, 0x00B8074A);
                SelectObject(dc, obr);
                DeleteObject(brush as _);
                SelectObject(mdc, ob);
                DeleteObject(mono as _);
                DeleteDC(mdc);
            }
            SelectObject(dc, of);
        });
    }
}

#[allow(dead_code)]
pub fn wide(s: &str) -> Vec<u16> {
    gdi::wide(s)
}
