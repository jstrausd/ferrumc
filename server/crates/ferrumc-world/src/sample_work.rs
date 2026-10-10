//! One owned sample allowance shared across actual density/spline/noise calls.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SampleWorkExceeded {
    pub limit: usize,
    pub used: usize,
}

pub(crate) struct SampleWork {
    remaining: usize,
    limit: usize,
    #[cfg(test)]
    density_trace: Option<Vec<&'static str>>,
    #[cfg(test)]
    trace_capacity: usize,
}

impl SampleWork {
    // Public callers validate their own point/batch/spline caps before this.
    pub(crate) const fn new(limit: usize) -> Self {
        Self {
            remaining: limit,
            limit,
            #[cfg(test)]
            density_trace: None,
            #[cfg(test)]
            trace_capacity: 0,
        }
    }

    pub(crate) fn charge(&mut self) -> Result<(), SampleWorkExceeded> {
        if self.remaining == 0 {
            return Err(SampleWorkExceeded {
                limit: self.limit,
                used: self.used(),
            });
        }
        self.remaining -= 1;
        Ok(())
    }

    pub(crate) const fn used(&self) -> usize {
        self.limit - self.remaining
    }
    #[cfg(test)]
    pub(crate) fn enable_density_trace(&mut self, capacity: usize) {
        assert!(capacity <= 4096);
        self.density_trace = Some(Vec::with_capacity(capacity));
        self.trace_capacity = capacity;
    }
    #[cfg(test)]
    pub(crate) fn record_density(&mut self, resource: &'static str) {
        if let Some(trace) = &mut self.density_trace {
            assert!(trace.len() < self.trace_capacity);
            trace.push(resource);
        }
    }
    #[cfg(test)]
    pub(crate) fn density_trace(&self) -> &[&'static str] {
        self.density_trace.as_deref().unwrap_or(&[])
    }
}
