# Fixed independent public-API contract fixtures

The FCPS3 records contain all46 FCIS slots, carried stack, position/look/selection,
negative zero, typed damage, ordered opaque component bytes and removal. Full
before/after TSV fixtures specify every actual row in all17 typed tables,
including unchanged tables, v1 bootstrap FCTR, image-CAS FCBR, joint FCBQ/FCBR,
complete full Chunk and chest-component bytes and exact scoped journal/meta.

`make_fixtures.py` is an offline fixed specification assembler. It imports no
FerrumC source/codec, reads no runtime database and is never called by Rust tests.
The tests consume these frozen bytes, independently read redb after dropping the
store, and compare the complete17-table maps. Agreement between two backends is
not the expected-value oracle. Root must review these authored fixtures separately;
this file does not claim an independent Root acceptance or Paper runtime result.
