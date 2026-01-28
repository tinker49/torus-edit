mod torus;

fn main() {
    // The main function catches any panics within `run_app_in_raw_mode` 
    // to observe the 'Original mode restored.' message printed by the Drop impl.
    // Without this, the panic handler might exit before the drop message prints,
    // but the mode is still restored before the process terminates.
    let result = std::panic::catch_unwind(|| {
        torus::terminal_handler::run_app_in_raw_mode();
    });

    if let Err(_err) = result {
        println!("A panic occurred, but the terminal mode was restored.");
    }
}

