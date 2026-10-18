//! One cooperative deadline shared by preparation and domain operations.
use crate::error::{AppError, Result};
use std::time::Instant;

pub fn check(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(AppError::new("DEADLINE", "Operation deadline exceeded.", 3))
    } else {
        Ok(())
    }
}
