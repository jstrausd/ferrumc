//! Exact Java-long distances, strict ties and explicitly retained leaf history.

use super::{
    core_charge as charge, core_fail as fail, limit, BiomeClimateTarget, BiomeCoreError,
    BiomeError, BiomeErrorKind, BiomeLimit, BiomeSelection, BiomeTree, NodeKind, SampleWork,
    MAX_BIOME_TREE_DEPTH,
};

pub(super) fn select(
    tree: &BiomeTree,
    target: BiomeClimateTarget,
    prior: Option<u16>,
    work: &mut SampleWork,
) -> Result<(u16, BiomeSelection), BiomeError> {
    select_core(tree, target, prior, work).map_err(crate::sample_work::CoreError::into_public)
}

pub(super) fn select_core(
    tree: &BiomeTree,
    target: BiomeClimateTarget,
    prior: Option<u16>,
    work: &mut SampleWork,
) -> Result<(u16, BiomeSelection), BiomeCoreError> {
    let q = target.coordinates();
    let coordinates = [q[0], q[1], q[2], q[3], q[4], q[5], 0];
    charge(work)?;
    let leaf = search(tree, tree.root, &coordinates, prior, work, 1)?
        .ok_or_else(|| fail(BiomeErrorKind::NoSelectableLeaf))?;
    let selection = tree.selection(leaf)?;
    charge(work)?;
    Ok((leaf, selection))
}
fn search(
    tree: &BiomeTree,
    node: u16,
    coordinates: &[i64; 7],
    prior: Option<u16>,
    work: &mut SampleWork,
    depth: usize,
) -> Result<Option<u16>, BiomeCoreError> {
    limit(BiomeLimit::Depth, depth, MAX_BIOME_TREE_DEPTH)?;
    charge(work)?;
    let node = tree
        .nodes
        .get(usize::from(node))
        .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
    match node.kind {
        NodeKind::Leaf(ordinal) => {
            let leaf = tree
                .leaf_nodes
                .get(usize::from(ordinal))
                .copied()
                .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
            Ok(Some(leaf))
        }
        NodeKind::Branch { children, len } => {
            if len == 0 || len > 6 {
                return Err(fail(BiomeErrorKind::InvalidTree));
            }
            let mut best_distance = if let Some(leaf) = prior {
                distance(tree, leaf, coordinates, work)?
            } else {
                i64::MAX
            };
            let mut best = prior;
            for &child in &children[..usize::from(len)] {
                charge(work)?;
                let child_distance = distance(tree, child, coordinates, work)?;
                charge(work)?;
                if best_distance > child_distance {
                    let candidate = search(tree, child, coordinates, best, work, depth + 1)?;
                    let leaf = candidate.ok_or_else(|| fail(BiomeErrorKind::NoSelectableLeaf))?;
                    let selected_distance = if child == leaf {
                        child_distance
                    } else {
                        distance(tree, leaf, coordinates, work)?
                    };
                    charge(work)?;
                    if best_distance > selected_distance {
                        best_distance = selected_distance;
                        best = Some(leaf);
                    }
                }
            }
            Ok(best)
        }
    }
}
fn distance(
    tree: &BiomeTree,
    node: u16,
    coordinates: &[i64; 7],
    work: &mut SampleWork,
) -> Result<i64, BiomeCoreError> {
    charge(work)?;
    let node = tree
        .nodes
        .get(usize::from(node))
        .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
    let precharged = work.try_charge_public_group(7);
    let mut sum = 0_i64;
    for ([minimum, maximum], &value) in node.axes.iter().zip(coordinates) {
        if !precharged {
            charge(work)?;
        }
        let high = value.wrapping_sub(*maximum);
        let low = minimum.wrapping_sub(value);
        let distance = if high > 0 { high } else { low.max(0) };
        sum = sum.wrapping_add(distance.wrapping_mul(distance));
    }
    Ok(sum)
}

#[cfg(test)]
mod throughput_tests {
    use super::*;
    use crate::biome_selection::{BiomeBuildBudget, MAX_BIOME_BUILD_WORK};
    use crate::sample_work::{CarverCumulativeMeter, Unsigned192};
    use ferrumc_registry::biome::BiomeParametersResourceId;

    fn original_distance(
        tree: &BiomeTree,
        node: u16,
        target: &[i64; 7],
        work: &mut SampleWork,
    ) -> Result<i64, BiomeCoreError> {
        charge(work)?;
        let node = tree
            .nodes
            .get(usize::from(node))
            .ok_or_else(|| fail(BiomeErrorKind::InvalidTree))?;
        let mut sum = 0_i64;
        for ([minimum, maximum], &value) in node.axes.iter().zip(target) {
            charge(work)?;
            let high = value.wrapping_sub(*maximum);
            let low = minimum.wrapping_sub(value);
            let distance = if high > 0 { high } else { low.max(0) };
            sum = sum.wrapping_add(distance.wrapping_mul(distance));
        }
        Ok(sum)
    }
    #[test]
    fn pure_distance_group_preserves_public_and_wide_refusal_boundaries() {
        let tree = BiomeTree::from_builtin(
            BiomeParametersResourceId::from_name("minecraft:overworld").unwrap(),
            BiomeBuildBudget::new(MAX_BIOME_BUILD_WORK).unwrap(),
        )
        .unwrap();
        let coordinates = [i64::MIN, i64::MAX, -99999, 99999, 0, 123, 0];
        for allowance in 0..=9 {
            let mut actual = SampleWork::new(allowance);
            let mut original = SampleWork::new(allowance);
            assert_eq!(
                distance(&tree, tree.root, &coordinates, &mut actual),
                original_distance(&tree, tree.root, &coordinates, &mut original)
            );
            assert_eq!(actual.used(), original.used());
            let make = || {
                SampleWork::with_carver_meter(CarverCumulativeMeter::new(Unsigned192::from_u64(
                    allowance as u64,
                )))
            };
            let mut actual = make();
            let mut original = make();
            assert_eq!(
                distance(&tree, tree.root, &coordinates, &mut actual),
                original_distance(&tree, tree.root, &coordinates, &mut original)
            );
            assert_eq!(actual.carver_meter().used(), original.carver_meter().used());
        }
    }
}
