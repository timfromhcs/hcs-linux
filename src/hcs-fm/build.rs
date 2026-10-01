fn main() {
    // One .slint entry per crate: include_modules!() only includes the last
    // build-script output. Shared widgets come from the hcs-ui kit by file path.
    slint_build::compile_with_config(
        "ui/fm.slint",
        slint_build::CompilerConfiguration::new()
            .with_include_paths(vec!["ui".into(), "../hcs-ui/ui".into()]),
    )
    .expect("fm.slint");
}
