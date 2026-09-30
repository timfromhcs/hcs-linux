//! `hcs-fm` — the Files binary.
//!
//! The GUI window is a thin layer: all listing, filtering and sorting lives in
//! the library and is unit-tested there. Interaction arrives as callbacks
//! because this Slint build has no touch-handler elements, which also keeps the
//! headless render deterministic.

use anyhow::Result;
use hcs_fm::{apply, human_date, human_size, DirectorySource, Filters, FsSource, SortSpec};
use hcs_ui::ThemePreset;
use std::path::PathBuf;

slint::include_modules!();

fn main() -> Result<()> {
    /// Keep the process alive briefly after doing its work, so an external sampler
    /// can measure real RSS instead of racing a process that exits immediately.
    /// Used by scripts/gui_ram_audit.py; harmless everywhere else.
    fn hold_for_measurement(args: &[String]) {
        if let Some(i) = args.iter().position(|a| a == "--hold-seconds") {
            if let Some(secs) = args.get(i + 1).and_then(|v| v.parse::<u64>().ok()) {
                std::thread::sleep(std::time::Duration::from_secs(secs.min(60)));
            }
        }
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let start = args
        .iter()
        .position(|a| a == "--path")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
        });

    let theme = args
        .iter()
        .position(|a| a == "--theme")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| ThemePreset::from_str_opt(s))
        .unwrap_or_else(hcs_ui::load_theme);

    // Machine-readable mode: the agent control surface requires every surface
    // to answer --json, so the CLI never depends on screen-scraping.
    if json {
        let entries = FsSource.list(&start)?;
        let sorted = apply(&entries, "", &Filters::name_only(), &SortSpec::default());
        println!(
            "{}",
            serde_json::json!({
                "path": start,
                "count": sorted.len(),
                "entries": sorted.iter().map(|e| serde_json::json!({
                    "name": e.name,
                    "dir": e.is_dir(),
                    "size": e.size_bytes,
                    "size_human": human_size(e.size_bytes),
                    "modified": e.modified,
                    "modified_human": human_date(e.modified),
                })).collect::<Vec<_>>(),
            })
        );
        return Ok(());
    }

    let window = FileManagerWindow::new()?;
    hcs_ui::apply_hcs_theme!(window, theme);
    window.set_cwd(start.to_string_lossy().to_string().into());
    window.set_active_tab_path(start.to_string_lossy().to_string().into());

    let entries = FsSource.list(&start)?;
    let sorted = apply(&entries, "", &Filters::name_only(), &SortSpec::default());
    let rows: Vec<FmEntry> = sorted
        .iter()
        .map(|e| FmEntry {
            name: e.name.clone().into(),
            is_dir: e.is_dir(),
            size_bytes: e.size_bytes.min(i32::MAX as u64) as i32,
            modified: human_date(e.modified).into(),
        })
        .collect();
    let entries_model = slint::VecModel::from(rows);
    // The activation handler needs the names again, so keep a plain copy
    // alongside the model rather than reading it back through the model API.
    let names: Vec<String> = sorted.iter().map(|e| e.name.clone()).collect();
    window.set_entries(slint::ModelRc::new(entries_model));
    window.set_status_line(format!("{} items", sorted.len()).into());

    // Interaction arrives as callbacks. Row activation runs through the same
    // default-program mapping the library exposes, so "Open" is one code path.
    {
        let weak = window.as_weak();
        let names = names.clone();
        window.on_item_activated(move |index| {
            if weak.upgrade().is_none() {
                return;
            }
            let Some(name) = names.get(index as usize) else {
                return;
            };
            let entry = hcs_fm::Entry {
                name: name.clone(),
                path: std::path::PathBuf::from(name),
                kind: hcs_fm::EntryKind::File,
                size_bytes: 0,
                modified: 0,
            };
            let target = match hcs_fm::default_program_for(&entry) {
                Some(c) => c.program,
                None => "hcs-notes".to_string(),
            };
            let _ = std::process::Command::new(&target).arg(name).spawn();
        });
    }
    {
        let weak = window.as_weak();
        window.on_open_with_requested(move || {
            if let Some(app) = weak.upgrade() {
                app.set_open_with_visible(true);
            }
        });
    }
    {
        let weak = window.as_weak();
        window.on_open_with_cancelled(move || {
            if let Some(app) = weak.upgrade() {
                app.set_open_with_visible(false);
            }
        });
    }
    {
        let weak = window.as_weak();
        window.on_open_with_chosen(move |program| {
            let Some(app) = weak.upgrade() else { return };
            app.set_open_with_visible(false);
            let target = app.get_open_with_target().to_string();
            let _ = std::process::Command::new(&*program).arg(&target).spawn();
        });
    }
    {
        let weak = window.as_weak();
        window.on_open_terminal(move || {
            let Some(app) = weak.upgrade() else { return };
            let cwd = app.get_cwd().to_string();
            let cmd = hcs_fm::terminal_command_for(std::path::Path::new(&cwd));
            app.set_terminal_cmd(cmd.clone().into());
            app.set_terminal_visible(true);
            let _ = std::process::Command::new("/usr/bin/hcs-term")
                .arg("-c")
                .arg(cmd)
                .spawn();
        });
    }
    {
        let weak = window.as_weak();
        window.on_new_tab(move || {
            if let Some(app) = weak.upgrade() {
                app.set_tab_count(app.get_tab_count() + 1);
            }
        });
    }
    {
        let weak = window.as_weak();
        window.on_view_toggled(move || {
            if let Some(app) = weak.upgrade() {
                app.set_list_view(!app.get_list_view());
            }
        });
    }
    {
        let weak = window.as_weak();
        window.on_filter_toggled(move |which| {
            let Some(app) = weak.upgrade() else { return };
            match which.as_str() {
                "name" => app.set_filter_name(!app.get_filter_name()),
                "content" => app.set_filter_content(!app.get_filter_content()),
                "modified" => app.set_filter_modified(!app.get_filter_modified()),
                _ => {}
            }
        });
    }

    window.run()?;
    hold_for_measurement(&args);
    Ok(())
}
