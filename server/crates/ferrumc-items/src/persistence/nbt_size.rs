//! Allocation-free NBT encoding preflight. The shared NBT writer validates
//! structure, but does not enforce all byte/string/sequence budgets on output.

use ferrumc_nbt::{NbtError, NbtLimits, NbtTag};

pub(super) fn file_root_size(tag: &NbtTag, limits: &NbtLimits) -> Result<usize, NbtError> {
    if !matches!(tag, NbtTag::Compound(_)) {
        return Err(NbtError::UnexpectedRootTag { id: tag_id(tag) });
    }
    // Root type plus the empty file-form root-name prefix.
    let mut budget = Budget { size: 0, limits };
    budget.add(3)?;
    budget.payload(tag, 1)?;
    Ok(budget.size)
}

struct Budget<'a> {
    size: usize,
    limits: &'a NbtLimits,
}

impl Budget<'_> {
    fn add(&mut self, amount: usize) -> Result<(), NbtError> {
        let size = self.size.saturating_add(amount);
        if size > self.limits.max_bytes() {
            return Err(NbtError::MaxBytesExceeded {
                len: size,
                max: self.limits.max_bytes(),
            });
        }
        self.size = size;
        Ok(())
    }

    fn sequence(&mut self, length: usize, element_size: usize) -> Result<(), NbtError> {
        if length > self.limits.max_list_len() {
            return Err(NbtError::ListTooLong {
                len: length,
                max: self.limits.max_list_len(),
            });
        }
        self.add(4)?;
        self.add(length.saturating_mul(element_size))
    }

    fn string(&mut self, text: &str) -> Result<(), NbtError> {
        let max = self.limits.max_string_bytes().min(usize::from(u16::MAX));
        let mut length = 0;
        // Modified UTF-8 counts NUL as two bytes and astral scalars as a
        // six-byte surrogate pair; ordinary UTF-8 length is not a safe budget.
        for ch in text.chars() {
            length += match u32::from(ch) {
                0x0001..=0x007f => 1,
                0 | 0x0080..=0x07ff => 2,
                0x0800..=0xffff => 3,
                _ => 6,
            };
            if length > max {
                return Err(NbtError::StringTooLong { len: length, max });
            }
        }
        self.add(2)?;
        self.add(length)
    }

    fn payload(&mut self, tag: &NbtTag, depth: usize) -> Result<(), NbtError> {
        match tag {
            NbtTag::Byte(_) => self.add(1),
            NbtTag::Short(_) => self.add(2),
            NbtTag::Int(_) | NbtTag::Float(_) => self.add(4),
            NbtTag::Long(_) | NbtTag::Double(_) => self.add(8),
            NbtTag::ByteArray(values) => self.sequence(values.len(), 1),
            NbtTag::IntArray(values) => self.sequence(values.len(), 4),
            NbtTag::LongArray(values) => self.sequence(values.len(), 8),
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

    fn list(&mut self, values: &[NbtTag], depth: usize) -> Result<(), NbtError> {
        self.check_depth(depth)?;
        // Validate the declared count before traversing a potentially large
        // server-authored tree, and bound every later append with `add`.
        self.add(1)?;
        self.sequence(values.len(), 0)?;
        let expected = values.first().map_or(0, tag_id);
        for value in values {
            let found = tag_id(value);
            if found != expected {
                return Err(NbtError::HeterogeneousList { expected, found });
            }
            self.payload(value, depth + 1)?;
        }
        Ok(())
    }

    fn check_depth(&self, depth: usize) -> Result<(), NbtError> {
        if depth > self.limits.max_depth() {
            return Err(NbtError::DepthExceeded {
                max: self.limits.max_depth(),
            });
        }
        Ok(())
    }
}

fn tag_id(tag: &NbtTag) -> u8 {
    match tag {
        NbtTag::Byte(_) => 1,
        NbtTag::Short(_) => 2,
        NbtTag::Int(_) => 3,
        NbtTag::Long(_) => 4,
        NbtTag::Float(_) => 5,
        NbtTag::Double(_) => 6,
        NbtTag::ByteArray(_) => 7,
        NbtTag::String(_) => 8,
        NbtTag::List(_) => 9,
        NbtTag::Compound(_) => 10,
        NbtTag::IntArray(_) => 11,
        NbtTag::LongArray(_) => 12,
    }
}
