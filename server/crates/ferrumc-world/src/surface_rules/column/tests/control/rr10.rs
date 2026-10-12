//! RR10-H2 explicit lava branch controls through the existing production cores.
use super::*;

#[test]
fn lava_does_not_stop_eroded_and_is_not_frozen_water() {
    let mut eroded = Bench::new(3, 0, |y| {
        if y == 90 {
            raw("lava")
        } else if y <= 80 {
            raw("stone")
        } else {
            raw("air")
        }
    });
    let mut expected = eroded.column(3, 0);
    original_eroded(&mut expected, 81, 100);
    eroded.run(&mut Silent, |s| {
        s.eroded_fill(3, 0, 81, 100).unwrap();
    });
    assert_eq!(eroded.column(3, 0), expected);
    assert_eq!(eroded.column(3, 0)[(90 - MIN_Y) as usize], raw("lava"));
    // Air above the lava still receives the eroded fill.
    assert_eq!(eroded.column(3, 0)[(91 - MIN_Y) as usize], raw("stone"));
    let mut frozen = Bench::new(0, 0, |y| if y == 50 { raw("lava") } else { raw("stone") });
    let before = frozen.column(0, 0);
    let mut random = WorldgenRandomSource::legacy(WorldSeed::new(4));
    let before_rng = random;
    frozen.run(&mut Silent, |s| {
        s.frozen_fill(
            0,
            0,
            60,
            40,
            FrozenShape {
                top: 65.0,
                bottom: 45.0,
            },
            4,
            60,
            &mut random,
        )
        .unwrap();
    });
    assert_eq!(frozen.column(0, 0), before);
    assert_eq!(random, before_rng);
}
