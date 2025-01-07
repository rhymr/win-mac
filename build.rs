use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    glib_build_tools::compile_resources(
        &["assets"],
        "assets/resources.xml",
        "compiled.gresource",
    );

    // Tell Cargo to rerun this if the resources change
    println!("cargo:rerun-if-changed=assets/resources.xml");
    println!("cargo:rerun-if-changed=assets/icons/text_dark.svg");
    
    // Print the output directory for debugging
    println!("cargo:warning=Resources compiled to: {}", out_dir.display());
} 