use std::process::ExitCode;

fn main() -> ExitCode {
    match cargo_spaced::cli::run() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}
