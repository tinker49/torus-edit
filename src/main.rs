mod app;
mod buffer;
mod config;
mod editor;
mod highlight;
mod input;
mod menu;
mod renderer;
mod search;

use std::io::{self, BufWriter};
use crossterm::{
    cursor,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() {
    // Parse file arguments before entering raw mode so errors print cleanly.
    let files: Vec<std::path::PathBuf> = std::env::args()
        .skip(1)
        .map(std::path::PathBuf::from)
        .collect();

    if let Err(e) = run(files) {
        eprintln!("ked: fatal error: {e}");
        std::process::exit(1);
    }
}

fn run(files: Vec<std::path::PathBuf>) -> std::io::Result<()> {
    // Install a panic hook that restores the terminal before printing the
    // backtrace — otherwise a crash leaves the user in a broken terminal state.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort cleanup.  Ignore errors: we're already panicking.
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stderr(),
            LeaveAlternateScreen,
            cursor::Show
        );
        default_hook(info);
    }));

    // Enter alternate screen and raw mode.
    enable_raw_mode()?;
    let mut stdout = BufWriter::new(io::stdout());
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    // Build renderer before the app so we know the terminal dimensions up front.
    let renderer = renderer::Renderer::new()?;
    let mut app  = app::App::new(renderer);

    // Open any files passed on the command line.
    for path in files {
        app.open_file(path);
    }

    // Run the main event loop.
    let result = app.run(&mut stdout);

    // Restore terminal regardless of whether the loop returned Ok or Err.
    let _ = execute!(stdout, LeaveAlternateScreen, cursor::Show);
    let _ = disable_raw_mode();

    result
}
