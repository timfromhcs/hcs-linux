fn main() {
    // One .slint entry per crate; shared widgets come from the hcs-ui kit by
    // file path.
    slint_build::compile_with_config(
        "ui/term.slint",
        slint_build::CompilerConfiguration::new()
            .with_include_paths(vec!["ui".into(), "../hcs-ui/ui".into()]),
    )
    .expect("term.slint");
}
