//! Literal production layouts on the pinned x86-64 toolchain; no cfg(test) fields.
use ferrumc_world::{
    biome_selection::BiomeSelectionSession,
    carver::{CarvedChunkBuffer, CarverError, CarverReport, PreparedCarverRun, SeededCarvers},
    density::NoiseChunkSession,
    surface_rules::{SeededSurfaceColumns, SurfaceChunkBuffer, SurfaceRuleSession},
};
use std::mem::size_of;
#[test]
fn production_values_are_literal_and_separate_from_retained_payloads() {
    println!("GEN0511_PRODUCTION_NATIVE binding={} prepared={} buffer={} noise={} selector={} rules={} columns={} surface={} error={} report={}",size_of::<SeededCarvers>(),size_of::<PreparedCarverRun<'_>>(),size_of::<CarvedChunkBuffer>(),size_of::<NoiseChunkSession>(),size_of::<BiomeSelectionSession>(),size_of::<SurfaceRuleSession<'_>>(),size_of::<SeededSurfaceColumns>(),size_of::<SurfaceChunkBuffer>(),size_of::<CarverError>(),size_of::<CarverReport>());
    assert_eq!(size_of::<PreparedCarverRun<'_>>(), 1360);
    assert_eq!(size_of::<NoiseChunkSession>(), 704);
    assert_eq!(size_of::<BiomeSelectionSession>(), 80);
    assert_eq!(size_of::<SurfaceRuleSession<'_>>(), 400);
    assert_eq!(size_of::<CarverError>(), 104);
    assert_eq!(size_of::<CarverReport>(), 104);
    assert_eq!(size_of::<SeededCarvers>(), 1560);
    assert_eq!(size_of::<CarvedChunkBuffer>(), 1104);
    assert_eq!(size_of::<SeededSurfaceColumns>(), 576);
    assert_eq!(size_of::<SurfaceChunkBuffer>(), 1088);
}
