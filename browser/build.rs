fn main() {
    // Puts the fox icon and version info into ZenkoBrowser.exe (only when building on Windows with MSVC).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        && cfg!(windows)
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("assets/zenko.ico", "1");
        res.set("ProductName", "Zenko Browser");
        res.set("FileDescription", "Zenko Browser");
        let _ = res.compile();
    }
    println!("cargo:rerun-if-changed=assets/zenko.ico");
    println!("cargo:rerun-if-changed=ui");
}
