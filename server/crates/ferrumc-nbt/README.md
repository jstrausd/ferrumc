# ferrumc-nbt

NBT parser/writer with depth, size, list length, and string byte limits. Fuzzed from day one.

`network_root_size` validates the complete encoded compound-root size without
encoding or allocating. Unlike the existing owned writers, it enforces every
configured output byte, sequence, and Modified UTF-8 string limit, with depth
capped at `min(configured, 512)`. It preserves ordered duplicate names and
float bits; empty network compounds have two bytes.

`write_network_root_into` appends that checked root directly through the
existing NBT/MUTF8 writer to a caller's already reserved `Vec<u8>`. It never
allocates, reserves, clones, or builds an intermediate buffer. Model errors
precede capacity errors, and any failure leaves the prefix, length, and
capacity unchanged. Success returns the appended root's length. The caller
owns fallible reservation and aggregate limits; encoded size is not a heap
budget. Existing owned writers retain their original validation and bytes.

## Invariants

See `INVARIANTS.md` in this directory.
