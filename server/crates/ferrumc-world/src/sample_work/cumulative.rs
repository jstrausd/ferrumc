//! Private exact run accounting. Segments never own producer state.

use std::cmp::Ordering;

/// Little-endian unsigned192; deliberately has no public arithmetic API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unsigned192([u64; 3]);

impl Unsigned192 {
    pub(crate) const ZERO: Self = Self([0; 3]);

    pub(crate) const fn from_limbs(limbs: [u64; 3]) -> Self {
        Self(limbs)
    }

    pub(crate) const fn from_u64(value: u64) -> Self {
        Self([value, 0, 0])
    }

    pub(crate) const fn limbs(self) -> [u64; 3] {
        self.0
    }

    pub(crate) fn checked_add(self, other: Self) -> Option<Self> {
        let mut output = [0; 3];
        let mut carry = false;
        for (index, word) in output.iter_mut().enumerate() {
            let (sum, first) = self.0[index].overflowing_add(other.0[index]);
            let (sum, second) = sum.overflowing_add(u64::from(carry));
            *word = sum;
            carry = first || second;
        }
        if carry {
            None
        } else {
            Some(Self(output))
        }
    }

    pub(crate) fn checked_sub(self, other: Self) -> Option<Self> {
        let mut output = [0; 3];
        let mut borrow = false;
        for (index, word) in output.iter_mut().enumerate() {
            let (difference, first) = self.0[index].overflowing_sub(other.0[index]);
            let (difference, second) = difference.overflowing_sub(u64::from(borrow));
            *word = difference;
            borrow = first || second;
        }
        if borrow {
            None
        } else {
            Some(Self(output))
        }
    }

    fn segment(self) -> u32 {
        if self.0[2] != 0 || self.0[1] != 0 || self.0[0] > u64::from(SEGMENT_UNITS) {
            SEGMENT_UNITS
        } else {
            self.0[0] as u32
        }
    }
}

impl Ord for Unsigned192 {
    fn cmp(&self, other: &Self) -> Ordering {
        for index in (0..3).rev() {
            let order = self.0[index].cmp(&other.0[index]);
            if order != Ordering::Equal {
                return order;
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Unsigned192 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) const SEGMENT_UNITS: u32 = 268_435_456;

/// Exact limit/used are the sole authority. Refilling only changes bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CarverCumulativeMeter {
    limit: Unsigned192,
    used: Unsigned192,
    segment_remaining: u32,
    #[cfg(test)]
    test_segment_size: u32,
}

impl CarverCumulativeMeter {
    pub(crate) fn new(limit: Unsigned192) -> Self {
        Self {
            limit,
            used: Unsigned192::ZERO,
            segment_remaining: limit.segment(),
            #[cfg(test)]
            test_segment_size: 0,
        }
    }

    pub(crate) const fn limit(&self) -> Unsigned192 {
        self.limit
    }
    pub(crate) const fn used(&self) -> Unsigned192 {
        self.used
    }

    pub(crate) fn remaining(&self) -> Unsigned192 {
        match self.limit.checked_sub(self.used) {
            Some(remaining) => remaining,
            None => unreachable!("private cumulative meter never exceeds its limit"),
        }
    }

    #[cfg(test)]
    pub(crate) fn test_position(
        &mut self,
        limit: Unsigned192,
        used: Unsigned192,
        segment_remaining: u32,
    ) {
        assert!(used <= limit);
        assert!(segment_remaining <= SEGMENT_UNITS);
        self.limit = limit;
        self.used = used;
        self.segment_remaining = segment_remaining;
    }
    #[cfg(test)]
    pub(crate) fn test_short_segments(&mut self) {
        self.test_segment_size = 1;
        self.segment_remaining = 0;
    }

    pub(crate) fn charge(&mut self) -> Result<(), super::ChargeFault> {
        // Refusals preserve every field, including segment bookkeeping.
        let next = self.used.checked_add(Unsigned192::from_u64(1));
        let Some(next) = next.filter(|next| *next <= self.limit) else {
            return Err(super::ChargeFault::Carver {
                limit: self.limit.limbs(),
                used: self.used.limbs(),
            });
        };
        if self.segment_remaining == 0 {
            self.segment_remaining = self.remaining().segment();
            #[cfg(test)]
            if self.test_segment_size != 0 {
                self.segment_remaining = self.segment_remaining.min(self.test_segment_size);
            }
        }
        self.used = next;
        self.segment_remaining -= 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_carries_borrows_order_and_overflow() {
        let one = Unsigned192::from_u64(1);
        for (before, after) in [
            ([u64::MAX, 0, 0], [0, 1, 0]),
            ([u64::MAX, u64::MAX, 0], [0, 0, 1]),
        ] {
            let before = Unsigned192::from_limbs(before);
            let after = Unsigned192::from_limbs(after);
            assert_eq!(before.checked_add(one), Some(after));
            assert_eq!(after.checked_sub(one), Some(before));
            assert!(after > before);
        }
        assert_eq!(
            Unsigned192::from_limbs([u64::MAX; 3]).checked_add(one),
            None
        );
        assert_eq!(Unsigned192::ZERO.checked_sub(one), None);
        assert!(
            Unsigned192::from_limbs([0, 0, 1]) > Unsigned192::from_limbs([u64::MAX, u64::MAX, 0])
        );
    }

    #[test]
    fn refusal_is_exact_and_mutation_free_even_at_192_overflow() {
        for limbs in [[0; 3], [u64::MAX; 3], [7, 3, 2]] {
            let mut meter = CarverCumulativeMeter::new(Unsigned192::from_limbs(limbs));
            meter.used = meter.limit;
            meter.segment_remaining = 0;
            let before = meter;
            assert_eq!(
                meter.charge(),
                Err(super::super::ChargeFault::Carver {
                    limit: limbs,
                    used: limbs
                })
            );
            assert_eq!(meter, before);
        }
    }

    #[test]
    fn segment_refill_retains_cumulative_authority() {
        let mut meter = CarverCumulativeMeter::new(Unsigned192::from_limbs([13, 2, 1]));
        meter.used = Unsigned192::from_limbs([u64::MAX, u64::MAX, 0]);
        meter.segment_remaining = 1;
        meter.charge().unwrap();
        assert_eq!(meter.used().limbs(), [0, 0, 1]);
        assert_eq!(meter.segment_remaining, 0);
        meter.charge().unwrap();
        assert_eq!(meter.used().limbs(), [1, 0, 1]);
        assert_eq!(meter.segment_remaining, SEGMENT_UNITS - 1);
        assert_eq!(meter.remaining().limbs(), [12, 2, 0]);
    }

    #[test]
    fn final_short_segment_ends_at_exact_limit() {
        let mut meter = CarverCumulativeMeter::new(Unsigned192::from_u64(3));
        for expected in 1..=3 {
            meter.charge().unwrap();
            assert_eq!(meter.used(), Unsigned192::from_u64(expected));
        }
        let before = meter;
        assert!(meter.charge().is_err());
        assert_eq!(meter, before);
    }
}
