use core::fmt;
use nalgebra::{DMatrix, DVector};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use serde_json as json;
use std::fmt::Debug;
use std::io::{self, IsTerminal, Read};
use std::iter::Iterator;
use std::{fs::File, path::Path};

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
    let matrix_size = compute_matrix_size(&parsed.input_matrix, &parsed.expression_rhs)
        .map_err(|err| NonInteractiveError::MatrixSizeError(err))?;

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

    let input_matrix = input_matrix.map_err(|err| NonInteractiveError::MatrixInputError(err))?;
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
        epsilon: Decimal::from_str(parsed.epsilon.as_str()).expect(DECIMAL_PARSE_ERROR_MESSAGE),
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

fn try_non_interactive() -> Result<EquesionInput, NonInteractiveError> {
    let content = try_file_path()
        .or_else(try_stdin)
        .map_or(Err(NonInteractiveError::NoInputProvided), |value| {
            value.map_err(|err| NonInteractiveError::IOError(err))
        })?;

    json::from_str::<EquesionInput>(content.as_str())
        .map_err(|err| NonInteractiveError::ParseError(err))
}

#[derive(Deserialize, Debug)]
struct EquesionInput {
    pub input_matrix: Vec<Vec<String>>,
    pub expression_rhs: Vec<String>,
    pub max_iterations: usize,
    pub epsilon: String,
}

fn try_stdin() -> Option<Result<String, io::Error>> {
    if io::stdin().lock().is_terminal() {
        return None;
    }

    let mut content = String::new();

    Some(io::stdin().read_to_string(&mut content).map(|_| content))
}

fn try_file_path() -> Option<Result<String, io::Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    let path = Path::new(arguments.get(1)?);
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) => return Some(Err(e)),
    };
    let mut content = String::new();
    Some(file.read_to_string(&mut content).map(|_| content))
}
