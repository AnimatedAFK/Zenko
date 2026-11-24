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
