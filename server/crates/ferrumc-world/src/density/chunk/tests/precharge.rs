//! RR11-M1 observations follow the actual production conversion and store.
use super::*;
use crate::sample_work::{CarverCumulativeMeter, Unsigned192};
use std::cell::Cell;

#[derive(Clone, Copy, Default)]
pub(crate) struct Conversions {
    pub(crate) len: usize,
    pub(crate) bits: [u32; 6],
}
thread_local! {
    static OBSERVER: Cell<Option<Conversions>> = const { Cell::new(None) };
}
// No answers or meter changes: the sole call site is after cast/store.
pub(in crate::density::chunk) fn observe_conversion(bits: u32) {
    OBSERVER.with(|observer| {
        if let Some(mut conversions) = observer.get() {
            assert!(conversions.len < 6);
            conversions.bits[conversions.len] = bits;
            conversions.len += 1;
            observer.set(Some(conversions));
        }
    });
}
pub(crate) fn observe<T>(run: impl FnOnce() -> T) -> (T, Conversions) {
    OBSERVER.with(|observer| {
        assert!(observer.replace(Some(Conversions::default())).is_none());
    });
    let result = run();
    let conversions = OBSERVER.with(|observer| observer.replace(None).unwrap());
    (result, conversions)
}
fn refusal(error: &NoiseChunkCoreError, limit: Unsigned192) {
    assert_eq!(
        error,
        &CoreError::Work(ChargeFault::Carver {
            limit: limit.limbs(),
            used: limit.limbs(),
        })
    );
}

#[test]
fn climate_precharge_nonfinite_error_order_and_refused_history() {
    // Controlled actual evaluator constants distinguish finite f64 -> f32
    // overflow from nonfinite f64 root rejection. These are not natural answers.
    for root in [f64::MAX, f64::INFINITY, f64::NAN] {
        for allowance in [1_u64, 2] {
            let mut session = super::wrappers::probe(
                vec![(
                    super::wrappers::original(crate::density::graph::NodeKind::Constant(root)),
                    "constant",
                )],
                0,
            );
            session.work = SampleWork::with_carver_meter(CarverCumulativeMeter::new(
                Unsigned192::from_u64(allowance),
            ));
            let before = session.carver_history();
            let (result, conversions) = observe(|| session.climate_sample(BlockPos::ORIGIN));
            let error = result.unwrap_err();
            if root.is_finite() && allowance == 1 {
                refusal(&error, Unsigned192::from_u64(1));
                assert_eq!(conversions.len, 0);
            } else {
                let CoreError::Error(error) = error else {
                    panic!("expected numerical error");
                };
                let NoiseChunkErrorKind::Density(density) = error.kind() else {
                    panic!("expected density error");
                };
                if root.is_finite() {
                    assert_eq!(conversions.len, 1);
                    assert_eq!(conversions.bits[0], f32::INFINITY.to_bits());
                    assert_eq!(
                        density.kind(),
                        &DensitySampleErrorKind::NonFiniteCoordinate {
                            bits: f32::INFINITY.to_bits()
                        }
                    );
                    assert_eq!(density.field(), "temperature");
                } else {
                    assert_eq!(conversions.len, 0);
                    assert_eq!(
                        density.kind(),
                        &DensitySampleErrorKind::NonFiniteDensity {
                            bits: root.to_bits()
                        }
                    );
                }
            }
            assert_eq!(session.carver_history(), before);
            let expected = if root.is_finite() { allowance } else { 1 };
            assert_eq!(session.carver_meter().used().limbs(), [expected, 0, 0]);
            println!("RR11_PRECHARGE defensive root={:016x} allowance={allowance} casts={} used={expected}", root.to_bits(), conversions.len);
        }
    }
}

#[test]
fn climate_precharge_public_refusal_and_nonfinite_order() {
    for root in [f64::MAX, f64::INFINITY, f64::NAN] {
        for allowance in [0_usize, 1, 2] {
            let mut session = super::wrappers::probe(
                vec![(
                    super::wrappers::original(crate::density::graph::NodeKind::Constant(root)),
                    "constant",
                )],
                0,
            );
            session.work = SampleWork::new(allowance);
            let history = session.carver_history();
            let (result, conversions) = observe(|| session.climate_sample(BlockPos::ORIGIN));
            let error = result.unwrap_err();
            if allowance == 0 {
                let NoiseChunkErrorKind::Density(density) = error.kind() else {
                    panic!("expected root work error");
                };
                assert_eq!(
                    density.kind(),
                    &DensitySampleErrorKind::WorkExceeded { limit: 0, used: 0 }
                );
                assert_eq!(conversions.len, 0);
            } else if root.is_finite() && allowance == 1 {
                assert_eq!(
                    error.kind(),
                    &NoiseChunkErrorKind::WorkExceeded {
                        limit: allowance,
                        used: allowance
                    }
                );
                assert_eq!(conversions.len, 0);
            } else {
                let NoiseChunkErrorKind::Density(density) = error.kind() else {
                    panic!("expected numerical error");
                };
                if root.is_finite() {
                    assert_eq!(
                        density.kind(),
                        &DensitySampleErrorKind::NonFiniteCoordinate {
                            bits: f32::INFINITY.to_bits()
                        }
                    );
                    assert_eq!(conversions.len, 1);
                } else {
                    assert_eq!(
                        density.kind(),
                        &DensitySampleErrorKind::NonFiniteDensity {
                            bits: root.to_bits()
                        }
                    );
                    assert_eq!(conversions.len, 0);
                }
            }
            assert_eq!(session.carver_history(), history);
            assert_eq!(
                session.work.used(),
                if root.is_finite() {
                    allowance
                } else {
                    allowance.min(1)
                }
            );
            println!(
                "RR11_PRECHARGE public root={:016x} allowance={allowance} casts={} used={}",
                root.to_bits(),
                conversions.len,
                session.work.used()
            );
        }
    }
}
