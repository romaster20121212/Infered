 
 

use std::path::{Path, PathBuf};

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        && std::env::var("PROFILE").as_deref() == Ok("release")
        && std::env::var_os("CARGO_FEATURE_DEVTEST").is_none()
    {
        println!("cargo:rustc-link-arg-bin=injector=/DEBUG:NONE");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../logo's/logo.png");

     
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let Some(ico) = render_icon() else {
        return;
    };

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("ico path is valid unicode"));
    if let Err(e) = res.compile() {
         
         
        println!("cargo:warning=could not embed the application icon: {e}");
    }
}

 
 
fn render_icon() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").ok()?);
    let png_path = manifest_dir.join("../logo's/logo.png");

    let img = image::open(&png_path)
        .map_err(|e| println!("cargo:warning=could not read {}: {e}", png_path.display()))
        .ok()?;

     
    let img = image::imageops::resize(
        &img.to_rgba8(),
        256,
        256,
        image::imageops::FilterType::Lanczos3,
    );

    let ico_path = Path::new(&std::env::var("OUT_DIR").ok()?).join("app.ico");
    let file = std::fs::File::create(&ico_path).ok()?;
    use image::ImageEncoder as _;
    image::codecs::ico::IcoEncoder::new(file)
        .write_image(img.as_raw(), 256, 256, image::ColorType::Rgba8.into())
        .map_err(|e| println!("cargo:warning=could not write the icon: {e}"))
        .ok()?;

    Some(ico_path)
}
