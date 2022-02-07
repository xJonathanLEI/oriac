use num_bigint::BigInt;

/// Maintains the resources of a Cairo run. Can be used across multiple runners.
#[derive(Debug)]
pub struct RunResources {
    pub n_steps: Option<BigInt>,
}

impl RunResources {
    /// Returns True if the resources were consumed.
    pub fn consumed(&self) -> bool {
        match &self.n_steps {
            Some(n_steps) => n_steps <= &BigInt::from(0),
            None => false,
        }
    }

    /// Consumes one Cairo step.
    pub fn consume_step(&mut self) {
        if let Some(n_steps) = &self.n_steps {
            self.n_steps = Some(n_steps - BigInt::from(1));
        }
    }
}
