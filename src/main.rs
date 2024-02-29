use std::fmt::Display;

use input::build_configuration;

use crate::{input::NonInteractiveError, solver::ESolveError};

mod input;
mod solver;

fn pad_string(displayable: impl Display, padding: usize) -> String {
    let string = displayable.to_string();
    string
        .lines()
        .map(|str| " ".repeat(padding) + str)
        .reduce(|acc, e| acc + "\n" + &e)
        .unwrap_or_default()
}

fn get_usage_information() -> &'static str {
    r#"solver <file-path>
solver < file-path

<file-path> is any valid path to a file
    "#
}

fn main() {
    let config = build_configuration();
    if let Err(err) = config {
        if let NonInteractiveError::NoInputProvided = err {
            eprintln!("{}", pad_string(get_usage_information(), 2));
            return;
        }
        eprintln!("Error occured:");
        eprintln!("{}", pad_string(err, 2));
        return;
    }

    let solution = config.unwrap().solve();
    match solution {
        Ok(result) => println!("Solution: {}", result),
        Err(error) => match error {
            ESolveError::Diverge => {
                eprintln!("Solution approximation diverges. Equesions do not have solution")
            }
        },
    }
}
