fn main() {
    // All shared Slint types live in a single .slint file and are exposed to
    // other crates as the Slint library `hcs_ui`. It must also be the *last*
    // file compiled, because `slint::include_modules!()` only includes the
    // last generated file (SLINT_INCLUDE_GENERATED is overwritten per compile).
    slint_build::compile_with_config(
        "ui/hcs_ui.slint",
        slint_build::CompilerConfiguration::new().as_library("hcs_ui"),
    )
    .expect("hcs_ui.slint");
}
