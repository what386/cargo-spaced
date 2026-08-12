use std::io;

use clap::{Command, Parser, ValueEnum};
use clap_complete::{Generator, Shell, generate};

#[derive(Parser, Debug)]
#[command(name = "completions", about = "Generate shell completions")]
struct Args {
    #[arg(value_enum)]
    shell: CompletionShell,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum CompletionShell {
    Bash,
    Elvish,
    Fish,
    Powershell,
    Zsh,
}

fn main() {
    let args = Args::parse();
    let mut command = Command::new("cargo-spaced");
    let mut output = io::stdout();

    match args.shell {
        CompletionShell::Bash => generate_completion(Shell::Bash, &mut command, &mut output),
        CompletionShell::Elvish => generate_completion(Shell::Elvish, &mut command, &mut output),
        CompletionShell::Fish => generate_completion(Shell::Fish, &mut command, &mut output),
        CompletionShell::Powershell => {
            generate_completion(Shell::PowerShell, &mut command, &mut output)
        }
        CompletionShell::Zsh => generate_completion(Shell::Zsh, &mut command, &mut output),
    }
}

fn generate_completion<G: Generator>(
    generator: G,
    command: &mut Command,
    output: &mut dyn io::Write,
) {
    generate(generator, command, "cargo-spaced", output);
}
