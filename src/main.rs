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

const USAGE_INFORMATION: &str = r#"solver <file-path>
solver < file-path

<file-path> is any valid path to a file
"#;

fn main() {
    let config = build_configuration();
    match config {
        Err(err) => {
            let error_string: &dyn Display = if err.is_no_input_provided() {
                &USAGE_INFORMATION
            } else {
                eprintln!("Error occured:");
                &err
            };

            eprintln!("{}", pad_string(error_string, 2));
        }
        Ok(config) => match config.solve() {
            Ok(result) => println!("Solution: {}", result),
            Err(error) => match error {
                ESolveError::Diverge => {
                    eprintln!("Solution approximation diverges. Equesions do not have solution")
                }
            },
        },
    }
}
