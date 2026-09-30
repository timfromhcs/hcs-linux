//! `hcs-term` — the terminal binary.
//!
//! Without a display server there is no window, but the binary stays useful: it
//! runs the profile's command and forwards its output. That is what keeps the
//! program scriptable and testable from a build host.

use anyhow::{bail, Context, Result};
use hcs_term::{builtin_profiles, search_profiles, Grid, ROWS};
use hcs_ui::ThemePreset;

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

    let profiles = builtin_profiles();
    let profile_arg = args
        .iter()
        .position(|a| a == "--profile")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let command = args
        .iter()
        .position(|a| a == "-c" || a == "--command")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let query = args
        .iter()
        .position(|a| a == "--profiles")
        .map(|i| args.get(i + 1).cloned().unwrap_or_default())
        .unwrap_or_default();

    if json || args.iter().any(|a| a == "--profiles") {
        let hits = search_profiles(&profiles, &query);
        println!(
            "{}",
            serde_json::json!({
                "query": query,
                "profiles": hits,
                "argv": profile_arg
                    .as_ref()
                    .and_then(|id| profiles.iter().find(|p| &p.id == id))
                    .map(|p| p.argv(command.as_deref()))
                    .unwrap_or_default(),
            })
        );
        return Ok(());
    }

    let profile = match profile_arg.as_ref() {
        Some(id) => match profiles.iter().find(|p| &p.id == id) {
            Some(p) => p.clone(),
            None => bail!(
                "unknown profile '{id}'. Available: {}",
                profiles
                    .iter()
                    .map(|p| p.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
        None => profiles[0].clone(),
    };

    let argv = profile.argv(command.as_deref());
    let headless = std::env::var("WAYLAND_DISPLAY").is_err() && std::env::var("DISPLAY").is_err();

    if headless {
        let status = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .with_context(|| format!("failed to launch '{}'", argv[0]))?;
        if !status.success() {
            bail!("'{}' exited with {status}", argv[0]);
        }
        return Ok(());
    }

    let theme = args
        .iter()
        .position(|a| a == "--theme")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| ThemePreset::from_str_opt(s))
        .unwrap_or_else(hcs_ui::load_theme);

    let window = TerminalWindow::new()?;
    hcs_ui::apply_hcs_theme!(window, theme);
    window.set_command_line(argv.join(" ").into());

    let mut grid = Grid::new(96, ROWS as usize);
    grid.write_str(&format!("hcs-term — {}\r\n", argv.join(" ")));
    set_rows(&window, &grid);

    // Run the profile's command and show its output in the grid.
    match std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
    {
        Ok(o) => {
            let mut g = Grid::new(96, ROWS as usize);
            g.write_str(&String::from_utf8_lossy(&o.stdout));
            g.write_str(&String::from_utf8_lossy(&o.stderr));
            set_rows(&window, &g);
            window.set_exit_code(o.status.code().unwrap_or(-1));
        }
        Err(e) => {
            let mut g = Grid::new(96, ROWS as usize);
            g.write_str(&format!("cannot launch '{}': {e}", argv[0]));
            set_rows(&window, &g);
            window.set_exit_code(127);
        }
    }

    // Profile picker: the GNOME 49 Ptyxis affordance.
    {
        let weak = window.as_weak();
        window.on_profile_toggled(move || {
            if let Some(app) = weak.upgrade() {
                app.set_profiles_visible(!app.get_profiles_visible());
            }
        });
    }
    {
        let weak = window.as_weak();
        window.on_profiles_cancelled(move || {
            if let Some(app) = weak.upgrade() {
                app.set_profiles_visible(false);
            }
        });
    }
    {
        let weak = window.as_weak();
        let names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();
        window.on_profile_chosen(move |name| {
            let Some(app) = weak.upgrade() else { return };
            app.set_profiles_visible(false);
            let Some(id) = names
                .iter()
                .position(|n| n == name.as_str())
                .and_then(|i| profiles.get(i))
            else {
                return;
            };
            let next = id.argv(None);
            let _ = std::process::Command::new(&next[0])
                .args(&next[1..])
                .spawn();
        });
    }

    window.run()?;
    hold_for_measurement(&args);
    Ok(())
}

fn set_rows(window: &TerminalWindow, grid: &Grid) {
    let rows: Vec<slint::SharedString> =
        (0..grid.rows()).map(|r| grid.row_text(r).into()).collect();
    window.set_rows(slint::ModelRc::new(slint::VecModel::from(rows)));
}
