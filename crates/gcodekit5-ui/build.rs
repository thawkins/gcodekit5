use chrono::Utc;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Generate build timestamp
    let build_date = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // Compile PO files
    let po_dir = Path::new("../../po");
    let out_dir = env::var("OUT_DIR").unwrap();
    let locale_dir = Path::new(&out_dir).join("locale");

    println!("cargo:rerun-if-changed=../../po");
    println!("cargo:rerun-if-changed=resources");

    if po_dir.exists() {
        for entry in fs::read_dir(po_dir).expect("Failed to read po dir") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "po") {
                let lang = path.file_stem().unwrap().to_string_lossy();
                let lang_dir = locale_dir.join(&*lang).join("LC_MESSAGES");
                fs::create_dir_all(&lang_dir).expect("Failed to create locale dir");

                let mo_path = lang_dir.join("gcodekit5.mo");

                let status = Command::new("msgfmt")
                    .arg("-o")
                    .arg(&mo_path)
                    .arg(&path)
                    .status();

                match status {
                    Ok(s) if s.success() => {}
                    _ => println!("cargo:warning=Failed to compile translations for {}", lang),
                }
            }
        }
    }

    // Expose the locale directory to the code
    println!("cargo:rustc-env=LOCALE_DIR={}", locale_dir.display());

    glib_build_tools::compile_resources(
        &["resources"],
        "resources/gresources.xml",
        "gcodekit5.gresource",
    );
}
