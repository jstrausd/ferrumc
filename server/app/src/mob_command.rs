//! Operator preview spawn requests; natural owner rules remain authoritative.
use ferrumc_math::Vec3;
use ferrumc_sim::MobKind;

pub(crate) const USAGE: &str = "usage: /mobspawn <sheep|cow|pig|chicken|zombie> <x> <y> <z>";

pub(crate) fn parse(args: &str) -> Option<(MobKind, Vec3)> {
    let mut args = args.split_whitespace();
    let name = args.next()?;
    let kind = match name.strip_prefix("minecraft:").unwrap_or(name) {
        "sheep" => MobKind::Sheep,
        "cow" => MobKind::Cow,
        "pig" => MobKind::Pig,
        "chicken" => MobKind::Chicken,
        "zombie" => MobKind::Zombie,
        _ => return None,
    };
    let position = Vec3::new(
        args.next()?.parse().ok()?,
        args.next()?.parse().ok()?,
        args.next()?.parse().ok()?,
    );
    if args.next().is_some()
        || !crate::player_data::is_valid_player_position(position)
        || !(-64.0..320.0).contains(&position.y)
    {
        return None;
    }
    Some((kind, position))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_complete_finite_supported_preview_proposals_parse() {
        assert_eq!(
            parse("cow 40.5 64 8.5"),
            Some((MobKind::Cow, Vec3::new(40.5, 64.0, 8.5)))
        );
        assert!(parse("minecraft:zombie -30 64 -0.5").is_some());
        for invalid in [
            "",
            "cow 1 64",
            "cow 1 64 1 extra",
            "cow NaN 64 1",
            "cow 1 inf 1",
            "cow 1 320 1",
            "cow 30000001 64 1",
            "cow ~ ~ ~",
            "creeper 1 64 1",
        ] {
            assert!(parse(invalid).is_none(), "{invalid}");
        }
    }
}
