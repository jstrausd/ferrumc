//! Allocation-bounded count planning before any component tree is cloned.

use super::{
    NativePlayerSnapshot, NormalSlotButton, PlayerOwnershipError, NATIVE_PLAYER_SLOT_COUNT,
};
use ferrumc_items::{encode_persisted_stack, plan_split, plan_transfer, ComponentValue, ItemStack};

#[derive(Clone, Copy)]
struct Cell<'a> {
    source: &'a ItemStack,
    count: u8,
    len: usize,
}

pub(crate) struct InventoryPlan<'a> {
    cells: [Cell<'a>; NATIVE_PLAYER_SLOT_COUNT + 1],
    growth_source: Option<&'a ItemStack>,
}

pub(crate) fn capacity(stack: &ItemStack) -> Result<u8, PlayerOwnershipError> {
    let Some(item) = stack.item() else {
        return Ok(0);
    };
    let mut cap = item.max_stack();
    for removed in stack.components().removed() {
        if matches!(removed.get(), 1..=3) {
            return Err(PlayerOwnershipError::UnresolvedComponentPolicy(
                removed.get(),
            ));
        }
    }
    for value in stack.components().added() {
        match value {
            ComponentValue::MaxStackSize(n) => cap = cap.min(*n),
            ComponentValue::Damage(d) if *d > 0 => cap = cap.min(1),
            ComponentValue::MaxDamage(_) => cap = cap.min(1),
            ComponentValue::Opaque(c) if matches!(c.type_id(), 1..=3) => {
                return Err(PlayerOwnershipError::UnresolvedComponentPolicy(c.type_id()))
            }
            _ => {}
        }
    }
    if stack.count() > cap {
        return Err(PlayerOwnershipError::Ineligible(
            "stack exceeds resolved native capacity",
        ));
    }
    Ok(cap)
}

fn same(a: &ItemStack, b: &ItemStack) -> Result<bool, PlayerOwnershipError> {
    if a.item().is_none() || a.item() != b.item() {
        return Ok(false);
    }
    // Canonical FCIS has a single fixed-width count after its resource name.
    // Validated persistence bytes preserve exact component order/float bits.
    let mut left = encode_persisted_stack(a)?;
    let mut right = encode_persisted_stack(b)?;
    let Some(item) = a.item() else {
        return Ok(false);
    };
    let count_offset = 9 + item.name().len();
    left[count_offset] = 0;
    right[count_offset] = 0;
    Ok(left == right)
}

impl<'a> InventoryPlan<'a> {
    pub(crate) fn new(
        slots: &'a [ItemStack; NATIVE_PLAYER_SLOT_COUNT],
        carried: &'a ItemStack,
    ) -> Result<Self, PlayerOwnershipError> {
        let placeholder = Cell {
            source: carried,
            count: 0,
            len: 7,
        };
        let mut cells = [placeholder; NATIVE_PLAYER_SLOT_COUNT + 1];
        for (cell, stack) in cells
            .iter_mut()
            .zip(slots.iter().chain(std::iter::once(carried)))
        {
            let bytes = encode_persisted_stack(stack)?;
            *cell = Cell {
                source: stack,
                count: stack.count(),
                len: bytes.len(),
            };
        }
        Ok(Self {
            cells,
            growth_source: None,
        })
    }

    pub(crate) fn drop_selected(
        &mut self,
        selected: u8,
        all: bool,
    ) -> Result<(&'a ItemStack, u8), PlayerOwnershipError> {
        let cell = &mut self.cells[36 + usize::from(selected)];
        capacity(cell.source)?;
        let moved = if all { cell.count } else { cell.count.min(1) };
        if moved == 0 {
            return Err(PlayerOwnershipError::Ineligible("empty selected hand"));
        }
        cell.count -= moved;
        Ok((cell.source, moved))
    }

    pub(crate) fn pickup(
        &mut self,
        selected: u8,
        source: &'a ItemStack,
    ) -> Result<u8, PlayerOwnershipError> {
        let limit = capacity(source)?;
        self.growth_source = Some(source);
        let mut remaining = source.count();
        let source_len = encode_persisted_stack(source)?.len();
        let order = std::iter::once(36 + usize::from(selected))
            .chain(std::iter::once(45))
            .chain(36..45)
            .chain(9..36);
        let mut visited = [false; NATIVE_PLAYER_SLOT_COUNT];
        for index in order {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            let cell = &mut self.cells[index];
            if cell.count == 0 || !same(cell.source, source)? {
                continue;
            }
            let dest_cap = capacity(cell.source)?.min(limit);
            let moved = remaining.min(dest_cap.saturating_sub(cell.count));
            cell.count += moved;
            remaining -= moved;
            if remaining == 0 {
                break;
            }
        }
        for index in (36..45).chain(9..36) {
            if remaining == 0 {
                break;
            }
            let cell = &mut self.cells[index];
            if cell.count != 0 {
                continue;
            }
            let moved = remaining.min(limit);
            *cell = Cell {
                source,
                count: moved,
                len: source_len,
            };
            remaining -= moved;
        }
        Ok(source.count() - remaining)
    }

    pub(crate) fn click(
        &mut self,
        slot: usize,
        button: NormalSlotButton,
    ) -> Result<(), PlayerOwnershipError> {
        if !(1..NATIVE_PLAYER_SLOT_COUNT).contains(&slot) || (5..9).contains(&slot) {
            return Err(PlayerOwnershipError::Ineligible(
                "recipe/armor/unknown slot click",
            ));
        }
        let mut dest = self.cells[slot];
        let mut cursor = self.cells[NATIVE_PLAYER_SLOT_COUNT];
        self.growth_source = Some(cursor.source);
        let right = button == NormalSlotButton::Right;
        if cursor.count == 0 {
            let moved = if right {
                dest.count.div_ceil(2)
            } else {
                dest.count
            };
            cursor = Cell {
                count: moved,
                ..dest
            };
            dest.count -= moved;
        } else if dest.count == 0 || same(dest.source, cursor.source)? {
            let cap = capacity(cursor.source)?;
            if dest.count != 0 {
                capacity(dest.source)?;
            }
            let moved = cursor
                .count
                .min(cap.saturating_sub(dest.count))
                .min(if right { 1 } else { u8::MAX });
            if dest.count == 0 {
                dest = Cell {
                    source: cursor.source,
                    count: 0,
                    len: cursor.len,
                };
            }
            dest.count += moved;
            cursor.count -= moved;
        } else {
            // Vanilla ordinary PICKUP swaps incompatible carried/clicked stacks
            // for either button when that slot accepts the whole carried count.
            capacity(cursor.source)?;
            std::mem::swap(&mut cursor, &mut dest);
        }
        self.cells[slot] = dest;
        self.cells[NATIVE_PLAYER_SLOT_COUNT] = cursor;
        Ok(())
    }

    pub(crate) fn encoded_len(&self) -> Result<usize, PlayerOwnershipError> {
        let len = self
            .cells
            .iter()
            .try_fold(super::NATIVE_PLAYER_HEADER_BYTES, |sum, c| {
                sum.checked_add(4 + if c.count == 0 { 7 } else { c.len })
                    .ok_or(PlayerOwnershipError::Capacity("planned player bytes"))
            })?;
        if len > ferrumc_storage::MAX_PLAYER_DATA_LEN {
            return Err(PlayerOwnershipError::Capacity("planned player bytes"));
        }
        Ok(len)
    }

    pub(crate) fn materialize(
        &self,
        position: ferrumc_math::Vec3,
        look: [f32; 2],
        mode: ferrumc_core::GameMode,
        selected: u8,
    ) -> Result<NativePlayerSnapshot, PlayerOwnershipError> {
        let mut slots = Box::new(std::array::from_fn(|_| ItemStack::empty()));
        for (stack, cell) in slots.iter_mut().zip(&self.cells) {
            *stack = cell.stack(self.growth_source)?;
        }
        let carried = self.cells[NATIVE_PLAYER_SLOT_COUNT].stack(self.growth_source)?;
        Ok(NativePlayerSnapshot::new(
            position, look, mode, selected, slots, carried,
        ))
    }
}

impl Cell<'_> {
    fn stack(self, growth_source: Option<&ItemStack>) -> Result<ItemStack, PlayerOwnershipError> {
        if self.count == 0 {
            return Ok(ItemStack::empty());
        }
        if self.count == self.source.count() {
            return Ok(self.source.clone());
        }
        if self.count > self.source.count() {
            let source =
                growth_source.ok_or(PlayerOwnershipError::Malformed("missing count source"))?;
            let plan = plan_transfer(
                source,
                self.source,
                self.count - self.source.count(),
                capacity(self.source)?,
            )?;
            if plan.destination_after().count() != self.count {
                return Err(PlayerOwnershipError::Malformed("count plan differs"));
            }
            return Ok(plan.into_stacks().1);
        }
        let plan = plan_split(self.source, self.count)?;
        Ok(plan.into_stacks().1)
    }
}
