fn main() {
    // Generate the build environment variables
    vergen::EmitBuilder::builder()
        .all_build()
        .all_cargo()
        .all_git()
        .all_rustc()
        .emit()
        .expect("Cannot set build environment variables");
}
