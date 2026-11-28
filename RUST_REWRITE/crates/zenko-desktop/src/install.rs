//! zenko.py's installing and updating on Windows (zenko15): the one-Zenko lock, cleanup after an update, the
//! first-run "Install Zenko?" question, --uninstall, install_zenko (copy, Start menu / desktop shortcuts, the Apps &
//! features entry), update_tick / update_check_now / update_install / update_restart / update_apply, Install from a
//! file, and the crash logs (a panic goes to error_log.txt like zk_log_error).
//!
//! While the Rust app is a preview it lives next to zenko.py's Zenko instead of replacing it: its own folder
//! (Programs\ZenkoRust), shortcut, Apps & features entry and log-in entry, and it only ever installs a
//! ZenkoRust.exe from a release (never zenko.py or zenko.py's Zenko.exe). Flip PREVIEW when it takes over.

use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_desktop::Companion;
use zenko_desktop::setpage::APP_VERSION;
use zenko_desktop::update;

pub const PREVIEW: bool = true;
const NAME: &str = if PREVIEW { "ZenkoRust" } else { "Zenko" };
const DETACHED: u32 = 0x0000_0008;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn install_dir() -> PathBuf {
    let la = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join("AppData").join("Local"));
    la.join("Programs").join(NAME)
}

fn exe_name() -> String {
    format!("{NAME}.exe")
}

fn app_file() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

fn same(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

pub fn is_installed() -> bool {
    app_file().parent().is_some_and(|d| same(d, &install_dir()))
}

fn uninstall_key() -> String {
    format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{NAME}")
}
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

// ---- the registry (HKEY_CURRENT_USER) -----------------------------------------------------------------------------

enum RegV<'a> {
    S(&'a str),
    D(u32),
}

fn reg_set(key: &str, values: &[(&str, RegV)]) -> Result<(), String> {
    unsafe {
        let mut h: HKEY = std::ptr::null_mut();
        let k = wide(key);
        if RegCreateKeyExW(HKEY_CURRENT_USER, k.as_ptr(), 0, std::ptr::null(), 0, KEY_WRITE, std::ptr::null(), &mut h,
                           std::ptr::null_mut()) != 0 {
            return Err("I couldn't write to the registry.".into());
        }
        for (n, v) in values {
            let nw = wide(n);
            match v {
                RegV::S(s) => {
                    let w = wide(s);
                    RegSetValueExW(h, nw.as_ptr(), 0, REG_SZ, w.as_ptr() as *const u8, (w.len() * 2) as u32);
                }
                RegV::D(d) => {
                    RegSetValueExW(h, nw.as_ptr(), 0, REG_DWORD, d as *const u32 as *const u8, 4);
                }
            }
        }
        RegCloseKey(h);
    }
    Ok(())
}

fn reg_get(key: &str, name: &str) -> Option<String> {
    unsafe {
        let (k, n) = (wide(key), wide(name));
        let mut buf = vec![0u16; 1024];
        let mut sz = (buf.len() * 2) as u32;
        if RegGetValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr() as _, &mut sz) != 0 {
            return None;
        }
        let l = buf.iter().position(|&c| c == 0).unwrap_or(0);
        Some(String::from_utf16_lossy(&buf[..l]))
    }
}

fn reg_del_value(key: &str, name: &str) {
    unsafe {
        let (k, n) = (wide(key), wide(name));
        RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr());
    }
}

/// win_folder(csidl): 0x10 desktop, 0x02 Start menu programs
fn win_folder(csidl: i32) -> PathBuf {
    let mut buf = [0u16; 520];
    unsafe { windows_sys::Win32::UI::Shell::SHGetFolderPathW(std::ptr::null_mut(), csidl, std::ptr::null_mut(), 0, buf.as_mut_ptr()) };
    let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
    PathBuf::from(String::from_utf16_lossy(&buf[..n]))
}

/// write_lnk(path, target, ...): the target's ANSI bytes in the PC's code page ("mbcs")
fn write_lnk(path: &Path, target: &Path, workdir: &Path, desc: &str) -> std::io::Result<()> {
    use windows_sys::Win32::Globalization::{CP_ACP, WideCharToMultiByte};
    let t = target.to_string_lossy().into_owned();
    let w: Vec<u16> = t.encode_utf16().collect();
    let mut a = vec![0u8; w.len() * 2 + 4];
    let n = unsafe { WideCharToMultiByte(CP_ACP, 0, w.as_ptr(), w.len() as i32, a.as_mut_ptr(), a.len() as i32, std::ptr::null(),
                                         std::ptr::null_mut()) };
    a.truncate(n.max(0) as usize);
    std::fs::write(path, update::lnk_bytes(&t, "", &workdir.to_string_lossy(), &t, desc, &a))
}

/// autostart_enabled / set_autostart for the installed exe (the per-user Run list)
pub fn autostart_enabled() -> bool {
    reg_get(RUN_KEY, NAME).is_some()
}

/// install_zenko(desktop, startmenu, autostart): the installed exe's path
pub fn install(desktop: bool, startmenu: bool, autostart: bool) -> Result<PathBuf, String> {
    let dir = install_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dst = dir.join(exe_name());
    let src = app_file();
    if !same(&src, &dst) {
        let old = dir.join(format!("{NAME}.old.exe"));
        let _ = std::fs::remove_file(&old);
        if dst.exists() && std::fs::rename(&dst, &old).is_err() {
            return Err("The installed Zenko is busy. Right-click the fox > Quit, then try again.".into());
        }
        std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
    }
    let lnk = format!("{NAME}.lnk");
    let desc = "Zenko, your fox focus buddy";
    if startmenu {
        let p = win_folder(0x02);
        if !p.as_os_str().is_empty() {
            let _ = write_lnk(&p.join(&lnk), &dst, &dir, desc);
        }
    }
    if desktop {
        let p = win_folder(0x10);
        if !p.as_os_str().is_empty() {
            let _ = write_lnk(&p.join(&lnk), &dst, &dir, desc);
        }
    }
    let size_kb = std::fs::metadata(&dst).map(|m| m.len() / 1024).unwrap_or(0) as u32;
    let (d, ds) = (dst.to_string_lossy().into_owned(), dir.to_string_lossy().into_owned());
    let un = format!("\"{d}\" --uninstall");
    let disp = if PREVIEW { "Zenko (Rust preview)" } else { "Zenko" };
    reg_set(&uninstall_key(), &[("DisplayName", RegV::S(disp)), ("DisplayVersion", RegV::S(APP_VERSION.trim_start_matches('v'))),
                                ("Publisher", RegV::S("Zenko")), ("DisplayIcon", RegV::S(&d)), ("InstallLocation", RegV::S(&ds)),
                                ("UninstallString", RegV::S(&un)), ("QuietUninstallString", RegV::S(&un)), ("NoModify", RegV::D(1)),
                                ("NoRepair", RegV::D(1)), ("EstimatedSize", RegV::D(size_kb))])?;
    if autostart {
        reg_set(RUN_KEY, &[(NAME, RegV::S(&format!("\"{d}\"")))])?;
    }
    Ok(dst)
}

/// uninstall_zenko(delete_data): shortcuts, Apps & features, the log-in entry; the folder goes a moment later
fn uninstall(delete_data: bool) {
    for c in [0x02, 0x10] {
        let _ = std::fs::remove_file(win_folder(c).join(format!("{NAME}.lnk")));
    }
    unsafe {
        let k = wide(&uninstall_key());
        RegDeleteKeyW(HKEY_CURRENT_USER, k.as_ptr());
    }
    reg_del_value(RUN_KEY, NAME);
    let mut targets = vec![install_dir()];
    if delete_data {
        targets.push(zenko_desktop::data_dir());
    }
    let cmd = format!("ping 127.0.0.1 -n 4 >nul & {}",
                      targets.iter().map(|t| format!("rmdir /s /q \"{}\"", t.display())).collect::<Vec<_>>().join(" & "));
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("cmd").arg("/c").raw_arg(&cmd)
        .current_dir(std::env::var_os("TEMP").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\")))
        .creation_flags(0x0800_0000 | DETACHED).spawn();
}

/// cleanup_after_update(): the previous exe an update left behind
pub fn cleanup_after_update() {
    if let Some(d) = app_file().parent() {
        let _ = std::fs::remove_file(d.join(format!("{NAME}.old.exe")));
    }
}

fn msg(text: &str, title: &str, flags: u32) -> i32 {
    let (t, c) = (wide(text), wide(title));
    unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), flags) }
}

static mut MUTEX: HANDLE = std::ptr::null_mut();

/// single_instance(): the same lock as zenko.py's, so zenko.py's Zenko and this one never run at once (they share
/// the data folder)
fn single_instance() -> bool {
    unsafe {
        let n = wide("FoxBuddy_SingleInstance_Mutex");
        let h = CreateMutexW(std::ptr::null(), 1, n.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if !h.is_null() {
                CloseHandle(h);
            }
            return false;
        }
        MUTEX = h;
        true
    }
}

fn zenko_running() -> bool {
    unsafe {
        let n = wide("FoxBuddy_SingleInstance_Mutex");
        let h = OpenMutexW(0x0010_0000, 0, n.as_ptr());
        if !h.is_null() {
            CloseHandle(h);
            return true;
        }
    }
    false
}

/// run_uninstaller(): what Apps & features runs
fn run_uninstaller() {
    let t = "Uninstall Zenko";
    if zenko_running() {
        msg("Zenko is still open. Right-click the fox > Quit, then try again.", t, MB_ICONINFORMATION);
        return;
    }
    let a = msg("Remove Zenko from this PC?\n\nYes = remove Zenko but keep my settings, notes and garden (handy if you reinstall)\n\
                 No = remove Zenko and delete all my Zenko data too", t, MB_YESNOCANCEL | MB_ICONQUESTION);
    if a == IDCANCEL {
        return;
    }
    uninstall(a == IDNO);
    msg("Zenko has been removed. Bye bye~ kon!", t, MB_ICONINFORMATION);
}

/// the first-run question (zenko.py's run_installer_window): 'installed', 'run' or 'quit'
fn run_installer() -> &'static str {
    let text = format!("Install Zenko\n{APP_VERSION} · free, no admin password needed\n\nZenko will live in your user folder, so it works on \
                        school PCs without asking IT:\n{}\n\nShe puts a shortcut on your desktop and in the Start menu. Your settings, \
                        notes and garden stay where they are. After this, Zenko can update herself (Settings > Updates).\n\n\
                        Yes = Install    No = Just run it    Cancel = Don't ask again (just run it)", install_dir().display());
    match msg(&text, "Install Zenko", MB_YESNOCANCEL | MB_ICONQUESTION) {
        IDYES => {
            if zenko_running() {
                msg("Zenko is already open. Right-click the fox > Quit, then press Install again.", "Install Zenko", MB_ICONINFORMATION);
                return "quit";
            }
            match install(true, true, autostart_enabled()) {
                Ok(dst) => {
                    use std::os::windows::process::CommandExt;
                    match std::process::Command::new(&dst).current_dir(install_dir()).creation_flags(DETACHED).spawn() {
                        Ok(_) => "installed",
                        Err(e) => {
                            msg(&format!("Installed, but I couldn't start it ({e}). Open it from the Start menu."), "Install Zenko", MB_ICONWARNING);
                            "quit"
                        }
                    }
                }
                Err(e) => {
                    msg(&format!("I couldn't install: {e}\nYou can still run it from here."), "Install Zenko", MB_ICONWARNING);
                    "run"
                }
            }
        }
        IDCANCEL => {
            let mut v = zenko_desktop::read_json("config.json");
            if !v.is_object() {
                v = serde_json::json!({});
            }
            v["install_skip"] = true.into();
            let _ = std::fs::create_dir_all(zenko_desktop::data_dir());
            let _ = std::fs::write(zenko_desktop::data_dir().join("config.json"), zenko_desktop::py_json_ascii(&v));
            "run"
        }
        _ => "run",
    }
}

/// main()'s start: --uninstall, cleanup, the first-run installer, the one-Zenko lock, the crash log.
/// false = don't start the app.
pub fn startup() -> bool {
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        update::log_error(&format!("{info}\n{bt}"), APP_VERSION);
    }));
    let args: Vec<String> = std::env::args().collect();
    let has = |a: &str| args.iter().any(|x| x == a);
    if has("--uninstall") {
        run_uninstaller();
        return false;
    }
    cleanup_after_update();
    if !is_installed() && !has("--portable") && !has("--updated") {
        let setup = app_file().file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().contains("setup"));
        let skip = zenko_desktop::read_json("config.json").get("install_skip").and_then(|v| v.as_bool()).unwrap_or(false);
        if (setup || !skip) && run_installer() != "run" {
            return false;
        }
    }
    let ok = if has("--updated") {
        let end = zenko_desktop::unix_now() + 25.0;
        loop {
            if single_instance() {
                break true;
            }
            if zenko_desktop::unix_now() > end {
                break false;
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
    } else {
        single_instance()
    };
    if !ok {
        msg("Zenko is already running (look for the fox near your clock).\n\nTo switch versions, right-click the fox > Quit, then \
             start this one again.", "Zenko", MB_ICONINFORMATION);
        return false;
    }
    true
}

// ---- updates ------------------------------------------------------------------------------------------------------

fn source(fox: &Companion) -> String {
    let s = fox.cfg.s("update_source", "");
    let s = s.trim();
    if s.is_empty() { "AnimatedAFK/Zenko".into() } else { s.into() }
}

static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn status(to_card: bool, fox: &mut Companion, m: &str, quiet: bool, secs: f64) {
    if to_card {
        crate::ears::set_msg_pub("update", m);
    } else if !quiet {
        fox.say(m, secs);
    }
}

/// update_tick (every minute): at most every 20 hours, quietly
pub fn tick(fox: &mut Companion, t: f64) {
    thread_local!(static NEXT: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) });
    if NEXT.with(|n| n.get()) == 0.0 {
        NEXT.with(|n| n.set(t + 60.0));
    }
    if t < NEXT.with(|n| n.get()) {
        return;
    }
    NEXT.with(|n| n.set(t + 60.0));
    if fox.cfg.s("update_mode", "auto") == "off" || BUSY.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    if zenko_desktop::unix_now() - fox.cfg.f("update_checked", 0.0) < 20.0 * 3600.0 {
        return;
    }
    fox.cfg_set("update_checked", serde_json::json!(zenko_desktop::unix_now()));
    check_now(fox, true, false);
}

/// update_check_now(quiet, status)
pub fn check_now(fox: &mut Companion, quiet: bool, to_card: bool) {
    let src = source(fox);
    BUSY.store(true, std::sync::atomic::Ordering::SeqCst);
    let exe = exe_name().to_lowercase();
    crate::voicehost::bg(move || update::update_check(&src, Some("exe"), &exe, false, 15.0), move |fox, _, r| {
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
        let info = match r {
            Err(e) => return status(to_card, fox, &e, quiet, 9.0),
            Ok(i) => i,
        };
        let Some(info) = info.filter(|i| update::version_newer(i["version"].as_str().unwrap_or(""), APP_VERSION)) else {
            return status(to_card, fox, &format!("You have the newest Zenko ({APP_VERSION})~"), quiet, 6.0);
        };
        let v = info["version"].as_str().unwrap_or("").to_string();
        if to_card {
            crate::ears::set_msg_pub("update", &format!("New version {v} is available."));
        }
        if quiet && v == fox.cfg.s("update_skip", "") {
            return;
        }
        if quiet && fox.cfg.s("update_mode", "auto") == "auto" && fox.session.is_none() && fox.lockin.is_none() {
            return install_update(fox, info, true);
        }
        *PENDING.lock().unwrap() = Some(info);
        fox.events.push(zenko_desktop::Ev::Say {
            text: format!("A new Zenko is ready ({v})! Update now? It takes a few seconds and keeps all your stuff."), secs: 14.0,
            choices: vec![("Update now".into(), "update_now".into()), ("Later".into(), String::new()), ("Skip it".into(), "update_skip".into())] });
    });
}

static PENDING: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

/// the bubble's Update now / Skip it
pub fn command(fox: &mut Companion, c: &str) -> bool {
    match c {
        "update_now" => {
            if let Some(i) = PENDING.lock().unwrap().take() {
                install_update(fox, i, false);
            }
        }
        "update_skip" => {
            if let Some(i) = PENDING.lock().unwrap().take() {
                fox.cfg_set("update_skip", i["version"].clone());
            }
        }
        _ => return false,
    }
    true
}

/// update_dest(): next to the exe when that folder is writable, else in the data folder
fn dest() -> PathBuf {
    let name = format!("{NAME}.new.exe");
    if let Some(d) = app_file().parent() {
        let test = d.join(".zenko_write_test");
        if std::fs::write(&test, b"").is_ok() {
            let _ = std::fs::remove_file(test);
            return d.join(name);
        }
    }
    let _ = std::fs::create_dir_all(zenko_desktop::data_dir());
    zenko_desktop::data_dir().join(name)
}

/// update_install(info, quiet)
fn install_update(fox: &mut Companion, info: serde_json::Value, quiet: bool) {
    if BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    if !quiet {
        fox.say("Downloading the new me... kon kon~", 8.0);
    }
    let d = dest();
    let i2 = info.clone();
    crate::voicehost::bg(move || update::update_download(&i2, &d, &mut |_| {}).map(|_| d), move |fox, _, r| {
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
        match r {
            Err(e) => {
                if !quiet {
                    fox.say(&e, 11.0);
                }
            }
            Ok(p) => restart(fox, &p, info["version"].as_str().unwrap_or("")),
        }
    });
}

/// update_apply(newfile): the running exe renamed (never deleted), the new one put in its place and started
fn apply(newfile: &Path) -> Result<(), String> {
    let cur = app_file();
    let dir = cur.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut old = dir.join(format!("{NAME}.old.exe"));
    if old.exists() && std::fs::remove_file(&old).is_err() {
        old = dir.join(format!("{NAME}.old{}.exe", zenko_desktop::unix_now() as i64));
    }
    std::fs::rename(&cur, &old).map_err(|e| e.to_string())?;
    let r = if newfile.parent().is_some_and(|p| same(p, &dir)) { std::fs::rename(newfile, &cur) } else { std::fs::copy(newfile, &cur).map(|_| ()) };
    if let Err(e) = r {
        let _ = std::fs::rename(&old, &cur); // put the old one back
        return Err(e.to_string());
    }
    use std::os::windows::process::CommandExt;
    std::process::Command::new(&cur).arg("--updated").current_dir(&dir).creation_flags(DETACHED).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

/// update_restart(newfile, version)
fn restart(fox: &mut Companion, newfile: &Path, version: &str) {
    if let Err(e) = apply(newfile) {
        fox.say(&format!("I couldn't swap in the update ({e}). Your current Zenko still works."), 11.0);
        return;
    }
    fox.say(&format!("Updating to {}... see you in a second!", if version.is_empty() { "the new version" } else { version }), 4.0);
    quit_soon(1.2);
}

fn quit_soon(secs: f64) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        crate::voicehost::post(|fox, _| fox.events.push(zenko_desktop::Ev::Command("quit".into())));
    });
}

/// Settings > Updates > Install from a file…
pub fn from_file(fox: &mut Companion, owner: HWND) {
    let Some(p) = crate::voicehost::open_file(owner, &format!("Pick the new {}", exe_name()), Path::new(""),
                                               &[(&exe_name(), "*.exe"), ("All files", "*.*")]) else { return };
    if same(&p, &app_file()) {
        return crate::ears::set_msg_pub("update", "That's the file I'm already running from~");
    }
    let (ok, why) = update::exe_ok(&p);
    if !ok {
        return crate::ears::set_msg_pub("update", &why);
    }
    restart(fox, &p, "");
}

/// Settings > Install Zenko on this PC (install_from_app)
pub fn from_app(fox: &mut Companion) {
    let dst = match install(true, true, autostart_enabled()) {
        Ok(d) => d,
        Err(e) => return crate::ears::set_msg_pub("update", &format!("I couldn't install: {e}")),
    };
    use std::os::windows::process::CommandExt;
    if let Err(e) = std::process::Command::new(&dst).arg("--updated").current_dir(install_dir()).creation_flags(DETACHED).spawn() {
        return crate::ears::set_msg_pub("update", &format!("Installed, but I couldn't restart from there ({e})."));
    }
    fox.say("Installed! Moving into my new home...", 4.0);
    quit_soon(1.2);
}
