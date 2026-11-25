// Windows-only WebView2 extras that wry does not wrap: browser-wide shortcuts, focus tracking,
// saved passwords, tab pictures for the command bar, mute, sleep and stop.
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use base64::Engine;
use tao::event_loop::EventLoopProxy;
use webview2_com::{
    AcceleratorKeyPressedEventHandler, CapturePreviewCompletedHandler, ContainsFullScreenElementChangedEventHandler, FocusChangedEventHandler,
    IsDocumentPlayingAudioChangedEventHandler, Microsoft::Web::WebView2::Win32::*, TrySuspendCompletedHandler,
};
use windows::Win32::{
    System::Com::{IStream, STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET},
    UI::{
        Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT},
        Shell::SHCreateMemStream,
    },
};
use windows_core::Interface;
use wry::{WebView, WebViewExtWindows};

use crate::Ev;

fn down(vk: u16) -> bool {
    unsafe { GetKeyState(vk as i32) < 0 }
}

/// "C+S+84" style name for a key press (C = Ctrl, S = Shift, A = Alt, then the Windows key code).
pub fn combo(vk: u32) -> String {
    let mut s = String::new();
    if down(VK_CONTROL.0) {
        s.push_str("C+");
    }
    if down(VK_SHIFT.0) {
        s.push_str("S+");
    }
    if down(VK_MENU.0) {
        s.push_str("A+");
    }
    s.push_str(&vk.to_string());
    s
}

pub fn hook(wv: &WebView, id: u32, proxy: EventLoopProxy<Ev>, keys: Rc<RefCell<HashSet<String>>>) {
    unsafe {
        let c = wv.controller();
        let mut tok = Default::default();
        let p = proxy.clone();
        let _ = c.add_AcceleratorKeyPressed(
            &AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                let Some(a) = args else { return Ok(()) };
                let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
                a.KeyEventKind(&mut kind)?;
                if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN || kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN {
                    let mut vk = 0u32;
                    a.VirtualKey(&mut vk)?;
                    if matches!(vk, 16 | 17 | 18) {
                        return Ok(());
                    }
                    let k = combo(vk);
                    if keys.borrow().contains(&k) {
                        a.SetHandled(true)?;
                        let _ = p.send_event(Ev::Key(k));
                    }
                }
                Ok(())
            })),
            &mut tok,
        );
        let p = proxy.clone();
        let _ = c.add_GotFocus(
            &FocusChangedEventHandler::create(Box::new(move |_, _| {
                let _ = p.send_event(Ev::Focus(id));
                Ok(())
            })),
            &mut tok,
        );
        let core = wv.webview();
        // Video full screen (YouTube etc.): the page asks, we make the window full screen.
        let p = proxy.clone();
        let _ = core.add_ContainsFullScreenElementChanged(
            &ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _| {
                if let Some(s) = sender {
                    let mut on = Default::default();
                    let _ = s.ContainsFullScreenElement(&mut on);
                    let _ = p.send_event(Ev::Fullscreen(id, on.as_bool()));
                }
                Ok(())
            })),
            &mut tok,
        );
        if let Ok(s) = core.Settings() {
            if let Ok(s4) = s.cast::<ICoreWebView2Settings4>() {
                let _ = s4.SetIsPasswordAutosaveEnabled(true);
                let _ = s4.SetIsGeneralAutofillEnabled(true);
            }
        }
        if let Ok(w8) = core.cast::<ICoreWebView2_8>() {
            let p = proxy.clone();
            let _ = w8.add_IsDocumentPlayingAudioChanged(
                &IsDocumentPlayingAudioChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(s) = sender {
                        if let Ok(s8) = s.cast::<ICoreWebView2_8>() {
                            let mut on = Default::default();
                            let _ = s8.IsDocumentPlayingAudio(&mut on);
                            let _ = p.send_event(Ev::Audio(id, on.as_bool()));
                        }
                    }
                    Ok(())
                })),
                &mut tok,
            );
        }
    }
}

fn read_stream(s: &IStream) -> Option<Vec<u8>> {
    unsafe {
        s.Seek(0, STREAM_SEEK_SET, None).ok()?;
        let mut st = STATSTG::default();
        s.Stat(&mut st, STATFLAG_NONAME).ok()?;
        let n = st.cbSize as usize;
        let mut buf = vec![0u8; n];
        let mut got = 0u32;
        s.Read(buf.as_mut_ptr() as *mut _, n as u32, Some(&mut got)).ok().ok()?;
        buf.truncate(got as usize);
        Some(buf)
    }
}

/// A JPEG of what the page shows right now, sent back as a data: URL.
pub fn capture(wv: &WebView, key: &str, proxy: EventLoopProxy<Ev>) {
    let key = key.to_string();
    unsafe {
        let Some(stream) = SHCreateMemStream(None) else {
            let _ = proxy.send_event(Ev::Shot(key, String::new()));
            return;
        };
        let s2 = stream.clone();
        let k2 = key.clone();
        let p2 = proxy.clone();
        let r = wv.webview().CapturePreview(
            COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG,
            &stream,
            &CapturePreviewCompletedHandler::create(Box::new(move |_hr| {
                let data = read_stream(&s2)
                    .map(|b| format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(b)))
                    .unwrap_or_default();
                let _ = p2.send_event(Ev::Shot(k2, data));
                Ok(())
            })),
        );
        if r.is_err() {
            let _ = proxy.send_event(Ev::Shot(key, String::new()));
        }
    }
}

pub fn mute(wv: &WebView, on: bool) {
    unsafe {
        if let Ok(w8) = wv.webview().cast::<ICoreWebView2_8>() {
            let _ = w8.SetIsMuted(on);
        }
    }
}

pub fn suspend(wv: &WebView) {
    unsafe {
        if let Ok(w3) = wv.webview().cast::<ICoreWebView2_3>() {
            let _ = w3.TrySuspend(&TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(()))));
        }
    }
}

pub fn stop(wv: &WebView) {
    unsafe {
        let _ = wv.webview().Stop();
    }
}

// ---------------------------------------------------------------- blocker, focus mode, link cleaner, crash recovery,
// and our own entries in the page's right-click menu.
use crate::guard::{self, Guard};
use webview2_com::{
    take_pwstr, CallDevToolsProtocolMethodCompletedHandler, ContextMenuRequestedEventHandler, CustomItemSelectedEventHandler,
    NavigationStartingEventHandler, ProcessFailedEventHandler, WebResourceRequestedEventHandler,
};
use windows_core::{HSTRING, PWSTR};

fn s_of(f: impl FnOnce(*mut PWSTR) -> windows_core::Result<()>) -> String {
    let mut p = PWSTR::null();
    if f(&mut p).is_ok() && !p.is_null() {
        take_pwstr(p)
    } else {
        String::new()
    }
}

pub fn extras(wv: &WebView, id: u32, proxy: EventLoopProxy<Ev>, guard: Rc<RefCell<Guard>>) {
    unsafe {
        let core = wv.webview();
        let env = wv.environment();
        let mut tok = Default::default();

        // ads and trackers
        let filtered = core
            .cast::<ICoreWebView2_22>()
            .and_then(|c| {
                c.AddWebResourceRequestedFilterWithRequestSourceKinds(
                    &HSTRING::from("*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                )
            })
            .is_ok();
        if !filtered {
            let _ = core.AddWebResourceRequestedFilter(&HSTRING::from("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL);
        }
        let g = guard.clone();
        let p = proxy.clone();
        let env2 = env.clone();
        let _ = core.add_WebResourceRequested(
            &WebResourceRequestedEventHandler::create(Box::new(move |sender, args| {
                let Some(a) = args else { return Ok(()) };
                let g = g.borrow();
                if !g.ads {
                    return Ok(());
                }
                let req = a.Request()?;
                let url = s_of(|p| req.Uri(p));
                let h = guard::host(&url);
                if h.is_empty() || !g.is_ad(&h) {
                    return Ok(());
                }
                if let Some(s) = sender {
                    let page = s_of(|p| s.Source(p));
                    let ph = guard::host(&page);
                    if g.ads_off.iter().any(|x| guard::host_matches(&ph, x)) || g.is_ad(&ph) {
                        return Ok(());
                    }
                }
                drop(g);
                if let Ok(resp) = env2.CreateWebResourceResponse(None, 403, &HSTRING::from("Blocked"), &HSTRING::from("")) {
                    let _ = a.SetResponse(&resp);
                    let _ = p.send_event(Ev::Blocked(id, h));
                }
                Ok(())
            })),
            &mut tok,
        );

        // focus mode + link cleaner (top-level navigations only)
        let g = guard.clone();
        let p = proxy.clone();
        let _ = core.add_NavigationStarting(
            &NavigationStartingEventHandler::create(Box::new(move |_, args| {
                let Some(a) = args else { return Ok(()) };
                let url = s_of(|p| a.Uri(p));
                if !url.starts_with("http") {
                    return Ok(());
                }
                let g = g.borrow();
                let h = guard::host(&url);
                if g.focus_blocks(&h) {
                    a.SetCancel(true)?;
                    let _ = p.send_event(Ev::Redirect(id, format!("zenko://blocked?u={}", enc(&url))));
                    return Ok(());
                }
                if g.clean {
                    if let Some(c) = guard::clean_url(&url) {
                        a.SetCancel(true)?;
                        let _ = p.send_event(Ev::Redirect(id, c));
                    }
                }
                Ok(())
            })),
            &mut tok,
        );

        // a crashed or frozen page reloads by itself
        let p = proxy.clone();
        let _ = core.add_ProcessFailed(
            &ProcessFailedEventHandler::create(Box::new(move |_, args| {
                if let Some(a) = args {
                    let mut k = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
                    a.ProcessFailedKind(&mut k)?;
                    if k == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
                        || k == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE
                        || k == COREWEBVIEW2_PROCESS_FAILED_KIND_FRAME_RENDER_PROCESS_EXITED
                    {
                        let _ = p.send_event(Ev::Crashed(id));
                    }
                }
                Ok(())
            })),
            &mut tok,
        );

        // our right-click entries
        if let (Ok(c11), Ok(env9)) = (core.cast::<ICoreWebView2_11>(), env.cast::<ICoreWebView2Environment9>()) {
            let p = proxy.clone();
            let _ = c11.add_ContextMenuRequested(
                &ContextMenuRequestedEventHandler::create(Box::new(move |_, args| {
                    let Some(a) = args else { return Ok(()) };
                    let items = a.MenuItems()?;
                    let t = a.ContextMenuTarget()?;
                    let mut b = Default::default();
                    // (label, command, data, in the Zenko submenu)
                    let mut add: Vec<(String, String, String, bool)> = vec![];
                    let _ = t.HasSelection(&mut b);
                    if b.as_bool() {
                        let sel = s_of(|p| t.SelectionText(p)).trim().to_string();
                        if !sel.is_empty() {
                            let short: String = sel.chars().take(24).collect();
                            let short = if sel.chars().count() > 24 { format!("{}...", short.trim_end()) } else { short };
                            if sel.split_whitespace().count() <= 4 {
                                add.push((format!("Look up \u{201c}{}\u{201d}", short), "lookup".into(), sel.clone(), false));
                            }
                            add.push((format!("Search the web for \u{201c}{}\u{201d}", short), "webSearch".into(), sel.clone(), false));
                            add.push(("Translate selection".into(), "translateSel".into(), sel.clone(), false));
                            add.push(("Copy as plain text".into(), "copyText".into(), sel.clone(), false));
                            add.push(("Count words".into(), "words".into(), sel.clone(), false));
                            add.push(("Summarize selection".into(), "sumSel".into(), sel.clone(), true));
                            add.push(("Make a flashcard".into(), "card".into(), sel.clone(), true));
                            add.push(("Add to Zenko (deadline or reminder)...".into(), "addZenko".into(), sel.clone(), true));
                            add.push(("Send to Zenko notes".into(), "toZenko".into(), sel.clone(), true));
                        }
                    }
                    let mut bl = Default::default();
                    let _ = t.HasLinkUri(&mut bl);
                    if bl.as_bool() {
                        let link = s_of(|p| t.LinkUri(p));
                        if link.starts_with("http") {
                            add.push(("Peek at link".into(), "peek".into(), link.clone(), false));
                            add.push(("Open link in split view".into(), "splitLink".into(), link.clone(), false));
                            add.push(("Open link in other profile".into(), "otherProfile".into(), link.clone(), false));
                            add.push(("Copy clean link".into(), "cleanLink".into(), link.clone(), false));
                            add.push(("Send link to Zenko notes".into(), "linkNote".into(), link.clone(), true));
                        }
                    }
                    let mut bs = Default::default();
                    let _ = t.HasSourceUri(&mut bs);
                    if bs.as_bool() && !b.as_bool() {
                        let src = s_of(|p| t.SourceUri(p));
                        if src.starts_with("http") {
                            add.push(("Open image in new tab".into(), "imgTab".into(), src, false));
                        }
                    }
                    let page = s_of(|p| t.PageUri(p));
                    if page.starts_with("http") && !bl.as_bool() && !b.as_bool() {
                        add.push(("Translate page".into(), "translatePage".into(), page.clone(), false));
                        add.push(("Cite this page (MLA / APA)".into(), "cite".into(), page.clone(), false));
                        add.push(("Copy page link".into(), "copyLink".into(), page.clone(), false));
                        add.push(("Save page for offline".into(), "savePage".into(), page.clone(), false));
                        add.push(("Summarize this page".into(), "sumPage".into(), page.clone(), true));
                        add.push(("Quiz me on this page".into(), "quizPage".into(), page.clone(), true));
                        add.push(("Clip part of this page".into(), "clip".into(), page.clone(), true));
                        add.push(("Send page to Zenko notes".into(), "pageNote".into(), page.clone(), true));
                    }
                    if add.is_empty() {
                        return Ok(());
                    }
                    let make = |label: &str, cmd: String, data: String| -> Option<ICoreWebView2ContextMenuItem> {
                        let item = env9.CreateContextMenuItem(&HSTRING::from(label), None, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_COMMAND).ok()?;
                        let p2 = p.clone();
                        let mut tk = Default::default();
                        let _ = item.add_CustomItemSelected(
                            &CustomItemSelectedEventHandler::create(Box::new(move |_, _| {
                                let _ = p2.send_event(Ev::Menu(id, cmd.clone(), data.clone()));
                                Ok(())
                            })),
                            &mut tk,
                        );
                        Some(item)
                    };
                    let mut at = 0u32;
                    let mut zk: Vec<ICoreWebView2ContextMenuItem> = vec![];
                    for (label, cmd, data, in_zenko) in add {
                        let Some(item) = make(&label, cmd, data) else { continue };
                        if in_zenko {
                            zk.push(item);
                        } else {
                            let _ = items.InsertValueAtIndex(at, &item);
                            at += 1;
                        }
                    }
                    // the Zenko submenu: everything that goes to Zenko in one place
                    if !zk.is_empty() {
                        if let Ok(sub) = env9.CreateContextMenuItem(&HSTRING::from("Zenko"), None, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SUBMENU) {
                            if let Ok(kids) = sub.Children() {
                                for (k, it) in zk.iter().enumerate() {
                                    let _ = kids.InsertValueAtIndex(k as u32, it);
                                }
                                let _ = items.InsertValueAtIndex(at, &sub);
                                at += 1;
                            }
                        }
                    }
                    if let Ok(sep) = env9.CreateContextMenuItem(&HSTRING::from(""), None, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SEPARATOR) {
                        let _ = items.InsertValueAtIndex(at, &sep);
                    }
                    Ok(())
                })),
                &mut tok,
            );
        }
    }
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{:02X}", b) })
        .collect()
}

/// Save the page as one .mhtml file (opens offline).
pub fn snapshot(wv: &WebView, id: u32, proxy: EventLoopProxy<Ev>) {
    unsafe {
        let _ = wv.webview().CallDevToolsProtocolMethod(
            &HSTRING::from("Page.captureSnapshot"),
            &HSTRING::from("{\"format\":\"mhtml\"}"),
            &CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |_hr, json| {
                let _ = proxy.send_event(Ev::Snapshot(id, json));
                Ok(())
            })),
        );
    }
}

pub fn zoom(wv: &WebView, f: f64) {
    unsafe {
        let _ = wv.controller().SetZoomFactor(f);
    }
}

/// On battery power?
pub fn on_battery() -> bool {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).is_ok() && s.ACLineStatus == 0 }
}

// ---------------------------------------------------------------- default browser + single window
pub fn register_default() -> bool {
    use windows::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
    let Ok(exe) = std::env::current_exe() else { return false };
    let exe = exe.to_string_lossy().to_string();
    let set = |key: &str, name: Option<&str>, val: &str| -> bool {
        let v: Vec<u16> = val.encode_utf16().chain(Some(0)).collect();
        let k = HSTRING::from(key);
        let n = name.map(HSTRING::from);
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &k,
                n.as_ref().map(|x| windows_core::PCWSTR(x.as_ptr())).unwrap_or(windows_core::PCWSTR::null()),
                REG_SZ.0,
                Some(v.as_ptr() as *const _),
                (v.len() * 2) as u32,
            )
            .is_ok()
        }
    };
    let cmd = format!("\"{}\" \"%1\"", exe);
    let app = "Software\\Clients\\StartMenuInternet\\ZenkoBrowser";
    let mut ok = set("Software\\Classes\\ZenkoBrowserHTML", None, "Zenko Browser page");
    ok &= set("Software\\Classes\\ZenkoBrowserHTML\\DefaultIcon", None, &format!("{},0", exe));
    ok &= set("Software\\Classes\\ZenkoBrowserHTML\\shell\\open\\command", None, &cmd);
    ok &= set(app, None, "Zenko Browser");
    ok &= set(&format!("{}\\DefaultIcon", app), None, &format!("{},0", exe));
    ok &= set(&format!("{}\\shell\\open\\command", app), None, &format!("\"{}\"", exe));
    let cap = format!("{}\\Capabilities", app);
    ok &= set(&cap, Some("ApplicationName"), "Zenko Browser");
    ok &= set(&cap, Some("ApplicationDescription"), "Your study browser, with Zenko.");
    for s in ["http", "https"] {
        ok &= set(&format!("{}\\URLAssociations", cap), Some(s), "ZenkoBrowserHTML");
    }
    for e in [".htm", ".html", ".pdf", ".mhtml", ".svg", ".webp"] {
        ok &= set(&format!("{}\\FileAssociations", cap), Some(e), "ZenkoBrowserHTML");
    }
    ok &= set("Software\\RegisteredApplications", Some("ZenkoBrowser"), &cap);
    let _ = std::process::Command::new("explorer").arg("ms-settings:defaultapps?registeredAppUser=ZenkoBrowser").spawn();
    ok
}

/// True when another Zenko Browser window is already running (it will pick up our links).
pub fn already_running() -> bool {
    use windows::Win32::{Foundation::GetLastError, System::Threading::CreateMutexW};
    unsafe {
        let h = CreateMutexW(None, false, &HSTRING::from("Local\\ZenkoBrowserOneWindow"));
        h.is_ok() && GetLastError() == windows::Win32::Foundation::ERROR_ALREADY_EXISTS
    }
}
