//! Windows only: embed the app icon and version details in `portmanteau.exe`.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        #[cfg(windows)]
        {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon/icon.ico");
            res.set("ProductName", "Portmanteau");
            res.set("FileDescription", "Portmanteau writing studio");
            if let Err(error) = res.compile() {
                println!("cargo:warning=could not embed the Windows icon: {error}");
            }
        }
    }
}
