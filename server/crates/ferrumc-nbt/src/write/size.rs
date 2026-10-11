//! Borrowed resource admission for the existing network-root writer.

use crate::mutf8;
use crate::{NbtError, NbtLimits, NbtTag, Result};

pub(super) fn network_root_size(tag: &NbtTag, limits: &NbtLimits) -> Result<usize> {
    let mut budget = Budget {
        size: 0,
        limits,
        depth_limit: limits.max_depth().min(NbtLimits::DEFAULT_MAX_DEPTH),
    };
    budget.check_depth(1)?;
    // Network NBT has one root type byte and no file-form root name.
    budget.add(1)?;
    budget.payload(tag, 1)?;
    Ok(budget.size)
}

struct Budget<'a> {
    size: usize,
    limits: &'a NbtLimits,
    depth_limit: usize,
}

impl Budget<'_> {
    fn add(&mut self, amount: usize) -> Result<()> {
        let size = self
            .size
            .checked_add(amount)
            .ok_or(NbtError::EncodedSizeOverflow)?;
        if size > self.limits.max_bytes() {
            return Err(NbtError::MaxBytesExceeded {
                len: size,
                max: self.limits.max_bytes(),
            });
        }
        self.size = size;
        Ok(())
    }

    fn sequence_size(&self, length: usize, element_size: usize) -> Result<usize> {
        let max = self.limits.max_list_len().min(super::I32_LEN_MAX);
        if length > max {
            return Err(NbtError::ListTooLong { len: length, max });
        }
        length
            .checked_mul(element_size)
            .and_then(|payload| payload.checked_add(4))
            .ok_or(NbtError::EncodedSizeOverflow)
    }

    fn string(&mut self, text: &str) -> Result<()> {
        let max = self.limits.max_string_bytes().min(super::U16_LEN_MAX);
        let length = mutf8::bounded_encoded_len(text, max)?;
        self.add(2)?;
        self.add(length)
    }

    fn payload(&mut self, tag: &NbtTag, depth: usize) -> Result<()> {
        match tag {
            NbtTag::Byte(_) => self.add(1),
            NbtTag::Short(_) => self.add(2),
            NbtTag::Int(_) | NbtTag::Float(_) => self.add(4),
            NbtTag::Long(_) | NbtTag::Double(_) => self.add(8),
            NbtTag::ByteArray(values) => self.add(self.sequence_size(values.len(), 1)?),
            NbtTag::IntArray(values) => self.add(self.sequence_size(values.len(), 4)?),
            NbtTag::LongArray(values) => self.add(self.sequence_size(values.len(), 8)?),
            NbtTag::String(text) => self.string(text),
            NbtTag::List(values) => self.list(values, depth),
            NbtTag::Compound(compound) => {
                self.check_depth(depth)?;
                for (name, value) in compound.iter() {
                    self.add(1)?;
                    self.string(name)?;
                    self.payload(value, depth + 1)?;
                }
                self.add(1)
            }
        }
    }

    fn list(&mut self, values: &[NbtTag], depth: usize) -> Result<()> {
        self.check_depth(depth)?;
        let header = self.sequence_size(values.len(), 0)?;
        self.add(1)?;
        self.add(header)?;
        let expected = values.first().map_or(0, NbtTag::type_id);
        for value in values {
            let found = value.type_id();
            if found != expected {
                return Err(NbtError::HeterogeneousList { expected, found });
            }
            self.payload(value, depth + 1)?;
        }
        Ok(())
    }

    fn check_depth(&self, depth: usize) -> Result<()> {
        if depth > self.depth_limit {
            return Err(NbtError::DepthExceeded {
                max: self.depth_limit,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_does_not_advance_the_size_budget() {
        let limits = NbtLimits::default().with_max_bytes(usize::MAX);
        let mut budget = Budget {
            size: usize::MAX,
            limits: &limits,
            depth_limit: 512,
        };
        assert_eq!(budget.add(1), Err(NbtError::EncodedSizeOverflow));
        assert_eq!(budget.size, usize::MAX);
    }

    #[test]
    fn sequence_prefix_bounds_and_arithmetic_refuse_before_traversal() {
        let limits = NbtLimits::default().with_max_list_len(usize::MAX);
        let budget = Budget {
            size: 0,
            limits: &limits,
            depth_limit: 512,
        };
        // Exercise the format ceiling without constructing a multi-GiB Vec.
        assert_eq!(
            budget.sequence_size(super::super::I32_LEN_MAX + 1, usize::MAX),
            Err(NbtError::ListTooLong {
                len: super::super::I32_LEN_MAX + 1,
                max: super::super::I32_LEN_MAX,
            })
        );
        assert_eq!(
            budget.sequence_size(1, usize::MAX),
            Err(NbtError::EncodedSizeOverflow)
        );
        assert_eq!(budget.size, 0);
    }
}
