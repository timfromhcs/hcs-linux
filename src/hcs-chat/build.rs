fn main() {
    // Exactly one .slint entry per crate: include_modules!() only includes the
    // last build-script output.
    //
    // The shared Neural Glass widgets are pulled in by file path from the
    // sibling hcs-ui crate. This is used instead of the Slint `@library`
    // mechanism because that one depends on an experimental compiler feature
    // and on cargo `links` env-var conventions that do not match a workspace
    // layout. The cost is that the widget code is compiled into each app
    // (~80 KB), which is irrelevant next to a 235 MB ISO.
    slint_build::compile_with_config(
        "ui/chat.slint",
        slint_build::CompilerConfiguration::new()
            .with_include_paths(vec!["ui".into(), "../hcs-ui/ui".into()]),
    )
    .expect("chat.slint");
}
