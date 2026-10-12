//! Actual RR11-M1 producer witnesses and conversion admission controls.
use super::*;
use crate::density::NoiseChunkCoreError;
use crate::density::{ClimateAxis, ClimateSample, NoiseChunkSession};

fn fresh(router: &SeededRouter) -> NoiseChunkSession {
    router
        .fresh_carver_noise_chunk(ChunkPos::ORIGIN, CarverCumulativeMeter::new(NOISE_TOTAL))
        .unwrap()
}
fn query(session: &mut NoiseChunkSession) -> Result<ClimateSample, NoiseChunkCoreError> {
    session.sample_carver_climate(CarverClimateQuart::source(ChunkPos::ORIGIN, -8, -8).unwrap())
}
fn refusal(error: NoiseChunkCoreError, limit: Unsigned192) {
    assert_eq!(
        noise_error(error).kind(),
        &CarverErrorKind::ProducerWorkExceeded {
            producer: CarverProducer::Noise,
            required: [1, 0, 0],
            limit: limit.limbs(),
            used: limit.limbs(),
        }
    );
}

#[test]
fn climate_precharge_review_witnesses_and_conversion_n_boundaries() {
    let router = crate::density::RouterTemplate::compile_builtin(
        ferrumc_registry::worldgen::NoiseSettingsResourceId::from_name("minecraft:overworld")
            .unwrap(),
    )
    .unwrap()
    .bind_seed(WorldSeed::new(0))
    .unwrap();
    for (allowance, prior_count, bits) in [(39, 0, 0x3d87_04f4), (59, 1, 0x3dc4_fc7f)] {
        // Each control constructs a genuine fresh producer, never resets a query.
        for extra in [0, 1, 2] {
            let mut session = fresh(&router);
            let used = session.carver_meter().used();
            assert_eq!(used.limbs(), [570_916, 0, 0]);
            let limit = used
                .checked_add(Unsigned192::from_u64(allowance + extra))
                .unwrap();
            session.carver_meter_mut().test_position(limit, used, 0);
            let (result, conversions) =
                NoiseChunkSession::observe_climate_conversions(|| query(&mut session));
            refusal(result.unwrap_err(), limit);
            assert_eq!(session.carver_meter().used(), limit);
            // The closed public query below checks poisoned lifecycle.
            assert_eq!(conversions.0, prior_count + usize::from(extra != 0),
                "a conversion whose charge refuses must not run: allowance={allowance}, extra={extra}");
            if extra != 0 {
                assert_eq!(conversions.1[prior_count], bits);
            }
            // A second request refuses lifecycle before touching meter or history.
            let meter = *session.carver_meter();
            let history = session.carver_history();
            let CoreError::Error(error) = query(&mut session).unwrap_err() else {
                panic!("expected poisoned lifecycle error");
            };
            assert_eq!(
                error.kind(),
                &crate::density::NoiseChunkErrorKind::Lifecycle {
                    operation: "sample_carver_climate",
                    phase: "poisoned"
                }
            );
            assert_eq!(*session.carver_meter(), meter);
            assert_eq!(session.carver_history(), history);
            println!(
                "RR11_PRECHARGE allowance={} casts={} bits={:x?} limit={:?}",
                allowance + extra,
                conversions.0,
                &conversions.1[..conversions.0],
                limit.limbs()
            );
        }
    }
    let mut session = fresh(&router);
    let used = session.carver_meter().used();
    let (sample, conversions) =
        NoiseChunkSession::observe_climate_conversions(|| query(&mut session));
    let sample = sample.unwrap();
    assert_eq!(conversions.0, 6);
    for axis in ClimateAxis::ALL {
        assert_eq!(conversions.1[axis as usize], sample.value(axis).to_bits());
    }
    let actual = session.carver_meter().used().checked_sub(used).unwrap();
    println!(
        "RR11_PRECHARGE successful-query work={:?} bits={:x?}",
        actual.limbs(),
        conversions.1
    );
    // Complete-query N-1/N/N+1, including exact successful history and bits.
    let expected_history = session.carver_history();
    for delta in [-1_i64, 0, 1] {
        let mut control = fresh(&router);
        let count = actual.limbs()[0].checked_add_signed(delta).unwrap();
        let limit = used.checked_add(Unsigned192::from_u64(count)).unwrap();
        control.carver_meter_mut().test_position(limit, used, 0);
        let (result, seen) = NoiseChunkSession::observe_climate_conversions(|| query(&mut control));
        if delta == -1 {
            refusal(result.unwrap_err(), limit);
            assert_eq!(seen.0, 5);
        } else {
            let value = result.unwrap();
            assert_eq!(seen.1, conversions.1);
            for axis in ClimateAxis::ALL {
                assert_eq!(value.value(axis).to_bits(), sample.value(axis).to_bits());
            }
            assert_eq!(control.carver_history(), expected_history);
            assert_eq!(control.carver_meter().used(), session.carver_meter().used());
        }
    }
}
