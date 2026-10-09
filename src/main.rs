use std::{io::IsTerminal, process::ExitCode};

fn main() -> ExitCode {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("TermXBoard requires an interactive terminal.");
        return ExitCode::FAILURE;
    }

    match termxboard::ui::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("TermXBoard failed: {error}");
            ExitCode::FAILURE
        }
    }
}
