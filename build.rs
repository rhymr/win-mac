fn main() {
    // let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    glib_build_tools::compile_resources(
        &["assets"],
        "assets/resources.xml",
        "compiled.gresource",
    );
} 