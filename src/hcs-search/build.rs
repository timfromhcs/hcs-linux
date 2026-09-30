fn main() {
    slint_build::compile_with_config(
        "ui/search.slint",
        slint_build::CompilerConfiguration::new()
            .with_include_paths(vec!["ui".into(), "../hcs-ui/ui".into()]),
    )
    .expect("search.slint");
}
