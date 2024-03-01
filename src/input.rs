use core::fmt;
use nalgebra::{DMatrix, DVector};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use serde_json as json;
use std::fmt::Debug;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::iter::Iterator;

use crate::solver::Equation;

const DECIMAL_PARSE_ERROR_MESSAGE: &str = "Can't represent such precise value";
const ZERO_ON_DIAGONAL_ERROR_MESSAGE: &str =
    "Zero on diagonal detected! Expected non-zero value on diagonal!";

fn build_decimal_from_string(input: &String) -> Result<Decimal, rust_decimal::Error> {
    Decimal::from_str(input)
}

pub fn build_configuration() -> Result<Equation, NonInteractiveError> {
    // try non interactive
    let parsed = try_non_interactive()?;
    let matrix_size = compute_matrix_size(&parsed.input_matrix, &parsed.expression_rhs)?;

    let input_matrix: Result<Vec<_>, _> = parsed
        .input_matrix
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, input)| {
            build_decimal_from_string(input).map_err(|err| PositionalError {
                row: index / matrix_size + 1,
                column: index % matrix_size + 1,
                message: err.to_string(),
            })
        })
        .collect();

    let input_matrix = input_matrix?;
    let matrix = DMatrix::from_row_iterator(matrix_size, matrix_size, input_matrix);
    check_for_zeroes_on_diagonal((&matrix, matrix_size))
        .map_err(|err| NonInteractiveError::MatrixInputError(err))?;

    let raw_expression_rhs = parsed
        .expression_rhs
        .iter()
        .enumerate()
        .map(|(index, v)| {
            build_decimal_from_string(v)
                .map_err(|err| NonInteractiveError::RightHandSideError(index + 1, err.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let expression_rhs: DVector<Decimal> = DVector::from_vec(raw_expression_rhs);

    Ok(Equation {
        input_matrix: matrix,
        expression_rhs,
        max_iterations: parsed.max_iterations,
        epsilon: Decimal::from_str(&parsed.epsilon).expect(DECIMAL_PARSE_ERROR_MESSAGE),
    })
}

/// Guass-Seidel method requires non-zero values on diagonal
/// source: https://www3.nd.edu/~zxu2/acms60212-40212-S12/Lec-09-4.pdf slide 10
fn check_for_zeroes_on_diagonal(matrix: (&DMatrix<Decimal>, usize)) -> Result<(), PositionalError> {
    let (matrix, matrix_size) = matrix;
    for i in 0..matrix_size {
        if matrix[(i, i)] == dec!(0) {
            let error = PositionalError {
                row: i,
                column: i,
                message: ZERO_ON_DIAGONAL_ERROR_MESSAGE.to_string(),
            };
            return Err(error);
        }
    }

    return Ok(());
}

fn compute_matrix_size(
    input_matrix: &Vec<Vec<String>>,
    expression_rhs: &Vec<String>,
) -> Result<usize, MatrixSizeError> {
    let row_sizes = input_matrix.into_iter().map(|row| row.len());
    let matrix_size = row_sizes
        .clone()
        .max()
        .ok_or(MatrixSizeError::EmptyMatrix)?;

    if let Some(incorrect_row) = row_sizes
        .clone()
        .enumerate()
        .find(|row_size| row_size.1 != matrix_size)
    {
        return Err(MatrixSizeError::WrongRowSize(
            WrongSize {
                actual: incorrect_row.1,
                expected: matrix_size,
            },
            // convert index to position
            incorrect_row.0 + 1,
        ));
    };

    let rows_amount = row_sizes.len();
    if rows_amount != matrix_size {
        return Err(MatrixSizeError::WrongRowsCount(WrongSize {
            actual: rows_amount,
            expected: matrix_size,
        }));
    };

    if expression_rhs.len() != matrix_size {
        return Err(MatrixSizeError::WrongExpressionRightHandSide(WrongSize {
            actual: expression_rhs.len(),
            expected: matrix_size,
        }));
    }

    Ok(matrix_size)
}

#[derive(Debug)]
pub struct WrongSize {
    actual: usize,
    expected: usize,
}

impl fmt::Display for WrongSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expected: {}! Got {}", self.expected, self.actual)
    }
}

#[derive(Debug)]
pub enum MatrixSizeError {
    WrongRowSize(WrongSize, usize),
    WrongExpressionRightHandSide(WrongSize),
    WrongRowsCount(WrongSize),
    EmptyMatrix,
}

impl fmt::Display for MatrixSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatrixSizeError::WrongRowSize(size, row_index) => {
                write!(f, "Row at position {row_index} has incorrect size: {size}")
            }
            MatrixSizeError::WrongExpressionRightHandSide(size) => {
                write!(f, "Expression right hand side size is incorrect: {size}")
            }
            MatrixSizeError::WrongRowsCount(size) => write!(f, "Rows count is incorrect: {size}"),
            MatrixSizeError::EmptyMatrix => write!(f, "Empty matrix provided!"),
        }
    }
}

#[derive(Debug)]
pub struct PositionalError {
    row: usize,
    column: usize,
    message: String,
}

#[derive(Debug)]
pub enum NonInteractiveError {
    MatrixSizeError(MatrixSizeError),
    MatrixInputError(PositionalError),
    RightHandSideError(usize, String),
    NoInputProvided,
    ParseError(serde_json::Error),
    IOError(io::Error),
}

impl From<MatrixSizeError> for NonInteractiveError {
    fn from(v: MatrixSizeError) -> Self {
        Self::MatrixSizeError(v)
    }
}

impl From<PositionalError> for NonInteractiveError {
    fn from(v: PositionalError) -> Self {
        Self::MatrixInputError(v)
    }
}

impl From<serde_json::Error> for NonInteractiveError {
    fn from(v: serde_json::Error) -> Self {
        Self::ParseError(v)
    }
}

impl NonInteractiveError {
    /// Returns `true` if the non interactive error is [`NoInputProvided`].
    ///
    /// [`NoInputProvided`]: NonInteractiveError::NoInputProvided
    #[must_use]
    pub fn is_no_input_provided(&self) -> bool {
        matches!(self, Self::NoInputProvided)
    }
}

impl From<io::Error> for NonInteractiveError {
    fn from(value: io::Error) -> Self {
        NonInteractiveError::IOError(value)
    }
}

impl fmt::Display for NonInteractiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonInteractiveError::MatrixSizeError(err) => {
                writeln!(f, "Incorrect input matrix sizing!")?;
                writeln!(f, "{}", err)
            }
            NonInteractiveError::NoInputProvided => writeln!(f, "No input provided!"),
            NonInteractiveError::ParseError(err) => {
                writeln!(f, "error during parsing occured! Error: {}", err)
            }
            NonInteractiveError::IOError(err) => {
                writeln!(f, "Unknown error occured! Error: {}", err)
            }
            NonInteractiveError::MatrixInputError(err) => {
                writeln!(
                    f,
                    "Incorrect value provided in {} row in {} column",
                    err.row, err.column
                )?;
                writeln!(f, "Error: {}", err.message)
            }
            NonInteractiveError::RightHandSideError(positon, message) => writeln!(
                f,
                "Incorrect value in right hand side expression on position {positon}! {}",
                message
            ),
        }
    }
}

enum InputMethod {
    Argument(String),
    Stdin,
    None,
}

impl From<Option<&str>> for InputMethod {
    fn from(value: Option<&str>) -> Self {
        value.map_or(InputMethod::None, |v| InputMethod::Argument(v.to_owned()))
    }
}

fn try_non_interactive() -> Result<EquesionInput, NonInteractiveError> {
    let content = match determine_input_method() {
        InputMethod::Argument(filepath) => fs::read_to_string(filepath),
        InputMethod::Stdin => read_from_stdin(),
        InputMethod::None => return Err(NonInteractiveError::NoInputProvided),
    }?;

    json::from_str::<EquesionInput>(&content).map_err(|err| err.into())
}

fn determine_input_method() -> InputMethod {
    let arguments: Vec<String> = std::env::args().collect();

    if let Some(filepath) = arguments.get(1) {
        return InputMethod::Argument(filepath.clone());
    }

    if !io::stdin().lock().is_terminal() {
        return InputMethod::Stdin;
    }

    return InputMethod::None;
}

#[derive(Deserialize, Debug)]
struct EquesionInput {
    pub input_matrix: Vec<Vec<String>>,
    pub expression_rhs: Vec<String>,
    pub max_iterations: usize,
    pub epsilon: String,
}

fn read_from_stdin() -> Result<String, io::Error> {
    let mut content = String::new();
    io::stdin().read_to_string(&mut content).map(|_| content)
}
