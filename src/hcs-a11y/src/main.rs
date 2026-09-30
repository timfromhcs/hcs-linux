//! `hcs-a11y-check` — the Gate 10 runner.
//!
//! Fails the build when a theme's text is not legible enough or a view has no
//! keyboard path. This is the check that `docs/GUI_BUILD_PLAN.md` §4 step 7
//! promised and that v1 never implemented.

use anyhow::{bail, Result};
use hcs_a11y::{check_contrast, theme_profiles, FocusOrder, FocusStop};

/// The declared focus order of every shipped view. A new view must be added
/// here; that requirement is the point — it turns "we support keyboards" from a
/// claim into a checked invariant.
fn focus_orders() -> Vec<FocusOrder> {
    let stop = |id: &str, tabbable: bool| FocusStop {
        id: id.to_string(),
        focusable: true,
        tabbable,
    };
    vec![
        FocusOrder {
            view: "omnibar".into(),
            stops: vec![
                stop("omnibar.input", true),
                stop("omnibar.results", true),
                stop("omnibar.quickkeys", false),
                stop("omnibar.preview", false),
            ],
        },
        FocusOrder {
            view: "taskbar".into(),
            stops: vec![
                stop("taskbar.start", true),
                stop("taskbar.search", true),
                stop("taskbar.windowlist", true),
                stop("taskbar.tray", true),
            ],
        },
        FocusOrder {
            view: "start_menu".into(),
            stops: vec![
                stop("menu.omnibar", true),
                stop("menu.pinned", true),
                stop("menu.all", true),
                stop("menu.power", true),
            ],
        },
        FocusOrder {
            view: "control_center".into(),
            stops: vec![
                stop("cc.tor", true),
                stop("cc.network", true),
                stop("cc.audio", true),
                stop("cc.brightness", true),
                stop("cc.profile", true),
            ],
        },
        FocusOrder {
            view: "notification_center".into(),
            stops: vec![
                stop("nc.dnd", true),
                stop("nc.list", true),
                stop("nc.clear", true),
            ],
        },
        FocusOrder {
            view: "files".into(),
            stops: vec![
                stop("fm.tabs", true),
                stop("fm.search", true),
                stop("fm.filter_name", true),
                stop("fm.filter_content", true),
                stop("fm.filter_date", true),
                stop("fm.entries", true),
                stop("fm.open_with", false),
            ],
        },
        FocusOrder {
            view: "terminal".into(),
            stops: vec![
                stop("term.tabs", true),
                stop("term.grid", true),
                stop("term.composer", true),
                stop("term.profiles", false),
            ],
        },
        FocusOrder {
            view: "notes".into(),
            stops: vec![
                stop("notes.list", true),
                stop("notes.editor", true),
                stop("notes.search", true),
            ],
        },
        FocusOrder {
            view: "docs".into(),
            stops: vec![
                stop("docs.manual_list", true),
                stop("docs.body", true),
                stop("docs.explain", true),
            ],
        },
        FocusOrder {
            view: "settings".into(),
            stops: vec![
                stop("settings.theme", true),
                stop("settings.keyboard", true),
                stop("settings.privacy", true),
                stop("settings.motion", true),
                stop("settings.scale", true),
                stop("settings.disclosure", true),
            ],
        },
        FocusOrder {
            view: "update".into(),
            stops: vec![
                stop("update.levels", true),
                stop("update.list", true),
                stop("update.apply", true),
                stop("update.rollback", true),
            ],
        },
        FocusOrder {
            view: "snap_flyout".into(),
            stops: vec![
                stop("snap.half_left", true),
                stop("snap.half_right", true),
                stop("snap.quarters", true),
                stop("snap.thirds", true),
                stop("snap.three_quarter", true),
            ],
        },
    ]
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let report_path = args
        .iter()
        .position(|a| a == "--report")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let mut failures: Vec<String> = Vec::new();

    // --- contrast ------------------------------------------------------------
    let mut contrast_results = Vec::new();
    for t in theme_profiles() {
        for (label, fg) in [("body", t.body_text), ("muted", t.muted_text)] {
            for (surface_label, bg) in [("bg", t.bg), ("surface", t.surface)] {
                let c = check_contrast(t.name, fg, bg, t.required_normal);
                if !c.passes {
                    failures.push(format!(
                        "contrast: theme {} {label} on {surface_label} is {:.2}:1, needs {:.1}:1",
                        t.name, c.ratio, t.required_normal
                    ));
                }
                contrast_results.push(c);
            }
        }
    }

    // --- focus order ---------------------------------------------------------
    let orders = focus_orders();
    let mut focus_results = Vec::new();
    for o in &orders {
        let problems = o.validate();
        for p in &problems {
            failures.push(format!("focus: {p}"));
        }
        focus_results.push(serde_json::json!({
            "view": o.view,
            "tabbable": o.tabbable_ids(),
            "problems": problems,
        }));
    }

    let report = serde_json::json!({
        "gate": "a11y",
        "status": if failures.is_empty() { "PASS" } else { "FAIL" },
        "themes_checked": theme_profiles().len(),
        "contrast_checks": contrast_results.len(),
        "views_checked": orders.len(),
        "contrast": contrast_results,
        "focus": focus_results,
        "failures": failures,
    });

    let rendered = serde_json::to_string_pretty(&report)?;
    if let Some(p) = report_path {
        std::fs::write(&p, format!("{rendered}\n"))
            .map_err(|e| anyhow::anyhow!("cannot write {p}: {e}"))?;
    }

    if json {
        println!("{rendered}");
    } else if failures.is_empty() {
        println!(
            "[PASS] accessibility gate: {} contrast checks across {} themes, {} views keyboard-reachable",
            contrast_results.len(),
            theme_profiles().len(),
            orders.len()
        );
    } else {
        for f in &failures {
            println!("  [FAIL] {f}");
        }
    }

    if !failures.is_empty() {
        bail!(
            "accessibility gate failed with {} problem(s)",
            failures.len()
        );
    }
    Ok(())
}
