//! `riffology`: the guitar game's window. `riffology --help` lists the options.

mod app;
mod args;
mod session;
mod trace;

/// Parses the options and opens the window.
fn main() -> eframe::Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match args::parse(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if args.help {
        println!("{}", args::HELP);
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([900.0, 560.0])
            .with_active(!args.background)
            .with_title("Riffology"),
        ..Default::default()
    };
    eframe::run_native(
        "riffology",
        options,
        Box::new(move |cc| Ok(Box::new(app::RiffologyApp::new(&cc.egui_ctx, args)))),
    )
}
