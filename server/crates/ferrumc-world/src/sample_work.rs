//! One owned sample allowance shared across actual density/spline/noise calls.

mod cumulative;
pub(crate) use cumulative::{CarverCumulativeMeter, Unsigned192};

/// A private fault keeps all limbs until the owning run's error adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChargeFault {
    Public { limit: usize, used: usize },
    Carver { limit: [u64; 3], used: [u64; 3] },
}

impl ChargeFault {
    // Only wrappers with publicly constructed work may use this adapter.
    pub(crate) fn into_public(self) -> SampleWorkExceeded {
        match self {
            Self::Public { limit, used } => SampleWorkExceeded { limit, used },
            Self::Carver { .. } => unreachable!("private run fault reached a public adapter"),
        }
    }
}

/// Existing numerical failures and exact private work faults remain distinct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CoreError<E> {
    Error(E),
    Work(ChargeFault),
}
impl<E> CoreError<E> {
    pub(crate) fn into_public(self) -> E {
        match self {
            Self::Error(error) => error,
            Self::Work(_) => unreachable!("private fault reached a public result adapter"),
        }
    }
}

enum Meter {
    Public { remaining: usize, limit: usize },
    Carver(CarverCumulativeMeter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SampleWorkExceeded {
    pub limit: usize,
    pub used: usize,
}

pub(crate) struct SampleWork {
    meter: Meter,
    #[cfg(test)]
    density_trace: Option<Vec<&'static str>>,
    #[cfg(test)]
    trace_capacity: usize,
}

impl SampleWork {
    // Public callers validate their own point/batch/spline caps before this.
    pub(crate) const fn new(limit: usize) -> Self {
        Self {
            meter: Meter::Public {
                remaining: limit,
                limit,
            },
            #[cfg(test)]
            density_trace: None,
            #[cfg(test)]
            trace_capacity: 0,
        }
    }

    pub(crate) fn charge(&mut self) -> Result<(), SampleWorkExceeded> {
        if !matches!(self.meter, Meter::Public { .. }) {
            unreachable!("private run cannot use a narrow charge adapter");
        }
        self.charge_core().map_err(ChargeFault::into_public)
    }

    pub(crate) const fn used(&self) -> usize {
        match &self.meter {
            Meter::Public { remaining, limit } => *limit - *remaining,
            Meter::Carver(_) => panic!("private counter has no usize representation"),
        }
    }

    pub(crate) const fn with_carver_meter(meter: CarverCumulativeMeter) -> Self {
        Self {
            meter: Meter::Carver(meter),
            #[cfg(test)]
            density_trace: None,
            #[cfg(test)]
            trace_capacity: 0,
        }
    }

    pub(crate) fn charge_core(&mut self) -> Result<(), ChargeFault> {
        match &mut self.meter {
            Meter::Public { remaining, limit } => {
                if *remaining == 0 {
                    return Err(ChargeFault::Public {
                        limit: *limit,
                        used: *limit,
                    });
                }
                *remaining -= 1;
                Ok(())
            }
            Meter::Carver(meter) => meter.charge(),
        }
    }

    pub(crate) fn carver_meter(&self) -> &CarverCumulativeMeter {
        match &self.meter {
            Meter::Carver(meter) => meter,
            Meter::Public { .. } => unreachable!("private counter requires admitted private work"),
        }
    }
    #[cfg(test)]
    pub(crate) fn carver_meter_mut(&mut self) -> &mut CarverCumulativeMeter {
        match &mut self.meter {
            Meter::Carver(meter) => meter,
            Meter::Public { .. } => unreachable!("private meter test requires private mode"),
        }
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
