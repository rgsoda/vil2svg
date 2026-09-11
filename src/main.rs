//! Draw a Vial .vil keymap as an SVG.
//!
//!     vil2svg corne.vil              -> corne.svg
//!     vil2svg corne.vil -o board.svg
//!     vil2svg corne.vil --open       -> also opens it in imv (macOS: open)
//!     vil2svg corne.vil --first-layer --print -> just the base layer, filling an A4 page
//!
//! Assumes a Corne / crkbd 3x6+3 matrix (LAYOUT_split_3x6_3). The physical layout is
//! fetched from QMK once and cached in ~/.cache/vil2svg, so later runs work offline.

mod cache;
mod config;
mod draw;
mod keymap;
mod layout;
mod legend;
mod png;
mod print_mode;
mod vil;

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "vil2svg", about = "Draw a Vial .vil keymap as an SVG.")]
struct Args {
    /// .vil file to draw
    input: PathBuf,

    /// output file (default: alongside the input)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// write a PNG instead of an SVG
    #[arg(long)]
    png: bool,

    /// PNG scale factor (default: 2, i.e. twice the SVG's nominal size)
    #[arg(long, default_value_t = 2.0, value_name = "N")]
    scale: f32,

    /// open the result when done
    #[arg(long)]
    open: bool,

    /// draw empty layers too
    #[arg(long)]
    all_layers: bool,

    /// draw only the first (base) layer, so it fills the page
    #[arg(long)]
    first_layer: bool,

    /// black on white, sized to A4
    #[arg(long = "print")]
    print_mode: bool,

    /// draw each combo as its own mini diagram instead of labelling it between
    /// the keys (clearer, but shrinks everything else on the page)
    #[arg(long)]
    combos_separate: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(out) => {
            println!("{}", out.display());
            if args.open {
                cache::open_svg(&out);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("vil2svg: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<PathBuf, String> {
    if !args.input.exists() {
        return Err(format!("{} not found", args.input.display()));
    }
    let out = args
        .output
        .clone()
        .unwrap_or_else(|| args.input.with_extension(if args.png { "png" } else { "svg" }));

    if args.png && !(args.scale.is_finite() && args.scale > 0.0) {
        return Err(format!("--scale must be a positive number, got {}", args.scale));
    }

    let parsed = vil::parse(&args.input, args.combos_separate)?;
    for s in &parsed.skipped_combos {
        eprintln!("vil2svg: skipped {s} — trigger keys not found on any layer");
    }

    let cfg = config::DrawConfig::for_vil2svg(args.combos_separate);
    let info_json = cache::cached_info_json()?;
    let physical =
        layout::PhysicalLayout::from_qmk_info_json(&info_json, "LAYOUT_split_3x6_3", cfg.key_h)?;
    if physical.len() != vil::N_KEYS {
        return Err(format!(
            "cached layout has {} keys, expected {}",
            physical.len(),
            vil::N_KEYS
        ));
    }

    // which layers to draw
    let selected: Vec<String> = if args.first_layer {
        parsed
            .keymap
            .layers
            .first()
            .map(|l| vec![l.name.clone()])
            .ok_or_else(|| format!("no layers found in {}", args.input.display()))?
    } else if args.all_layers {
        Vec::new()
    } else {
        parsed
            .keymap
            .layers
            .iter()
            .filter(|l| !l.name.starts_with("Empty"))
            .map(|l| l.name.clone())
            .collect()
    };

    let mut drawer = draw::Drawer::new(&cfg, &physical);
    let mut svg = drawer.print_board(&parsed.keymap, &selected);

    if args.print_mode {
        svg = print_mode::make_printable(&svg, 24)?;
    }

    let bytes = if args.png {
        png::render(&svg, args.scale)?
    } else {
        svg.into_bytes()
    };
    std::fs::write(&out, bytes).map_err(|e| format!("could not write {}: {e}", out.display()))?;
    Ok(out)
}
