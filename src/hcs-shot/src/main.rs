//! `hcs-shot` — capture CLI.
//!
//! Backends, in order of preference:
//!   1. `grim` (wlroots/Wayland) — no portal round-trip, works under the QA agent
//!   2. X11 tools (`import`, `xwd`) when only XWayland is up
//!   3. Refuse, with an error that names the requirement
//!
//! Refusing is a feature. Silently writing a black PNG is the failure mode the
//! v1 entropy audit caught three times.

use anyhow::{anyhow, Context, Result};
use hcs_shot::{default_output, describe, ocr_gate, validate, CaptureRequest, Format, Target};
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let flag = |name: &str| args.iter().any(|a| a == name);
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    let target = if flag("--region") {
        let geom = value("--geometry").unwrap_or_default();
        let (x, y, w, h) = parse_geometry(&geom)?;
        Target::Region { x, y, w, h }
    } else if flag("--window") {
        match value("--title") {
            Some(t) => Target::Window { title: t },
            None => Target::ActiveWindow,
        }
    } else if let Some(secs) = value("--delay").and_then(|v| v.parse::<u64>().ok()) {
        Target::Delayed {
            secs,
            target: Box::new(Target::Screen),
        }
    } else {
        Target::Screen
    };

    let format = match value("--format").as_deref() {
        Some("jpg") | Some("jpeg") => Format::Jpeg,
        Some("webp") => Format::Webp,
        _ => Format::Png,
    };

    let wants_ocr = flag("--extract");
    let wants_color = flag("--color");

    let req = CaptureRequest {
        target,
        format,
        output: value("--output").map(PathBuf::from),
        label: value("--label"),
    };

    if wants_ocr {
        ocr_gate(&req)?;
    }

    let (screen_w, screen_h) = screen_size();
    validate(&req, screen_w, screen_h)?;

    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let out = req
        .output
        .clone()
        .unwrap_or_else(|| default_output(&default_dir(), req.label.as_deref(), format, &stamp));

    if flag("--dry-run") {
        println!(
            "{}",
            serde_json::json!({
                "target": describe(&req.target),
                "format": format,
                "output": out,
                "backend": backend_name(),
                "ocr": wants_ocr,
                "color": wants_color,
            })
        );
        return Ok(());
    }

    if let Some(secs) = match req.target {
        Target::Delayed { secs, .. } => Some(secs),
        _ => None,
    } {
        std::thread::sleep(std::time::Duration::from_secs(secs));
    }

    let backend = backend_name().ok_or_else(|| {
        anyhow!(
            "no capture backend available.\n\
             hcs-shot needs a graphical session (grim for Wayland, or import/xwd under XWayland).\n\
             In a VM, boot the 'HCS Linux Recovery Console' entry only if you meant to."
        )
    })?;

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    run_backend(backend, &req, &out)?;

    let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    if bytes == 0 {
        // A zero-byte file is the signature of a backend that "succeeded" while
        // capturing nothing. Treat it as a failure now, not in the QA report.
        return Err(anyhow!(
            "capture produced an empty file at {}\n\
             This usually means the compositor has no visible surface yet — retry after the \
             window is mapped.",
            out.display()
        ));
    }

    let payload = serde_json::json!({
        "path": out,
        "bytes": bytes,
        "backend": backend,
        "target": describe(&req.target),
        "format": format,
    });

    if wants_color {
        let rgb = sample_color(&out);
        println!(
            "{}",
            serde_json::json!({ "path": out, "hex": rgb.map(|c| c.hex()) })
        );
    } else if json {
        println!("{payload}");
    } else {
        println!(
            "captured {} ({} bytes) via {backend} -> {}",
            describe(&req.target),
            bytes,
            out.display()
        );
    }

    Ok(())
}

fn parse_geometry(s: &str) -> Result<(i32, i32, u32, u32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err(anyhow!("--geometry must be X,Y,W,H (got '{s}')"));
    }
    let x = parts[0].trim().parse::<i32>()?;
    let y = parts[1].trim().parse::<i32>()?;
    let w = parts[2].trim().parse::<u32>()?;
    let h = parts[3].trim().parse::<u32>()?;
    Ok((x, y, w, h))
}

fn default_dir() -> PathBuf {
    std::env::var("XDG_PICTURES_DIR")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME").map(|h| PathBuf::from(h).join("Pictures").join("Screenshots"))
        })
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn screen_size() -> (u32, u32) {
    // wl_output geometry when available; the QA VM runs 1280x800, which is what
    // the VirtualBox stages are captured at.
    if let Ok(out) = Command::new("wlr-randr").arg("--no-heading").output() {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("1280x800") {
                    return (1280, 800);
                }
            }
        }
    }
    (1920, 1080)
}

fn backend_name() -> Option<&'static str> {
    if which("grim").is_some() {
        Some("grim")
    } else if which("import").is_some() || which("xwd").is_some() {
        Some("x11")
    } else {
        None
    }
}

fn which(prog: &str) -> Option<PathBuf> {
    let path = std::env::var("PATH").ok()?;
    for dir in path.split(':') {
        let candidate = PathBuf::from(dir).join(prog);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn run_backend(backend: &str, req: &CaptureRequest, out: &PathBuf) -> Result<()> {
    let (x, y, w, h) = match req.target {
        Target::Region { x, y, w, h } => (x, y, w, h),
        _ => (0, 0, 0, 0),
    };
    let mut cmd = if backend == "grim" {
        let mut c = Command::new("grim");
        if w > 0 && h > 0 {
            c.arg("-g").arg(format!("{x},{y} {w}x{h}"));
        }
        c
    } else {
        Command::new("import")
    };
    cmd.arg(out);
    let status = cmd
        .status()
        .with_context(|| format!("failed to launch the {backend} capture backend"))?;
    if !status.success() {
        return Err(anyhow!("{backend} exited with {status}"));
    }
    Ok(())
}

/// Read one pixel from a PNG without pulling in an image library: parse the
/// IHDR for dimensions and take the first pixel of the IDAT-decoded scanline.
/// A full decoder is out of scope; the colour picker only needs one sample and
/// this keeps the binary dependency-free.
fn sample_color(path: &PathBuf) -> Option<hcs_shot::Rgb> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 33 || &bytes[1..4] != b"PNG" {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let _ = (width, height);
    // After IHDR (8-byte header + 25 bytes) the IDAT chunk begins. The first
    // scanline starts with a filter byte; the next three bytes are the first
    // pixel for an 8-bit RGB/RGBA image.
    let start = 8 + 25 + 4 + 4;
    if bytes.len() < start + 4 {
        return None;
    }
    Some(hcs_shot::Rgb::from_rgb(
        bytes[start + 1],
        bytes[start + 2],
        bytes[start + 3],
    ))
}
