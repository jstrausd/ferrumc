# Independent FCPS3 input

`rich_app_layout.bin` fixes the native App scenario from
`server/app/src/player_data/tests.rs`, `rich_patch` and
`schema_three_preserves_all_slots_cursor_component_variants_and_float_bits`.
It was framed independently with Python `struct`, UTF-16-unit modified UTF-8
and literal FCIS/NBT grammar before any Sim decoder roundtrip.

Header41: `FCPS`, version3, position(-0.0,64.25,-3.5), look(-0.0,0.0),
selected8, slots46. Frames0..45 contain stone counts1..46, cursor count47.
All frames retain seven typed/opaque components, duplicate NBT keys, NaN bits
`7fc00011`, signed-zero double, embedded NUL/emoji and removals51/95.

Length10616; SHA256
`e624803678cdb1fa10293400e3e4094a6bbb756009c6947cdf7d3ed9cc6ee801`.
This is an independently framed parser input matching the pinned App scenario;
it is not a started Java/Paper gameplay differential run.
