use nalgebra::{DMatrix, DVector};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug)]
pub struct Equation {
    pub input_matrix: DMatrix<Decimal>,
    pub expression_rhs: DVector<Decimal>,
    pub max_iterations: usize,
    pub epsilon: Decimal,
}

pub enum ESolveError {
    Diverge,
}

impl Equation {
    pub fn solve(&self) -> Result<DVector<Decimal>, ESolveError> {
        let matrix_size = self.input_matrix.column_iter().count();
        let mut k: usize = 1;
        let mut result_vector = DVector::from_element(matrix_size, dec!(1));

        loop {
            let mut delta = dec!(0);

            for i in 0..matrix_size {
                let mut s = dec!(0);

                for j in 0..matrix_size {
                    // skip current line
                    if j == i {
                        continue;
                    }
                    s += self.input_matrix[(i, j)] * result_vector[j];
                }

                let x = (self.expression_rhs[i] - s) / self.input_matrix[(i, i)];
                let d = (x - result_vector[i]).abs();
                if d > delta {
                    delta = d;
                }

                result_vector[i] = x;
            }

            if delta < self.epsilon {
                return Ok(result_vector);
            }

            if k < self.max_iterations {
                k += 1;
                continue;
            }

            return Err(ESolveError::Diverge);
        }
    }
}
