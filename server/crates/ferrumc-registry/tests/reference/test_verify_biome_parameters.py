#!/usr/bin/env python3
"""Meaningful offline pin/schema refusal guards, also active under Python -O."""

import copy
import tempfile
import unittest
from pathlib import Path

import verify_biome_parameters as oracle


class ReferenceGuards(unittest.TestCase):
    def test_all_frozen_producer_inputs_and_ordered_data_pass(self):
        self.assertEqual(oracle.verify(), 7598)

    def test_all_seven_adopted_pins_reject_byte_and_digest_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input"
            for relative, pin in oracle.PINS.items():
                raw = (oracle.CRATE / relative).read_bytes()
                path.write_bytes(raw)
                self.assertEqual(oracle.read_pinned(path, *pin), raw)
                path.write_bytes(bytes([raw[0] ^ 1]) + raw[1:])
                with self.assertRaisesRegex(RuntimeError, "SHA256"):
                    oracle.read_pinned(path, *pin)
                path.write_bytes(raw + b" ")
                with self.assertRaisesRegex(RuntimeError, "byte count"):
                    oracle.read_pinned(path, *pin)

    def test_duplicate_decoded_keys_and_nonfinite_tokens_refuse(self):
        for raw in (b'{"a":1,"\\u0061":2}', b'{"a":NaN}', b'{"a":Infinity}'):
            with self.assertRaises(RuntimeError):
                oracle.parse(raw)

    def test_order_reference_count_and_exact_integer_domains_refuse(self):
        parameters = oracle.parse((oracle.CRATE / "data/biome-parameters-1.21.8.json").read_bytes())
        keys = {row["biome"] for rows in parameters.values() for row in rows}
        for field, changed in (("index", 1), ("index", False), ("biome", "nether_wastes"),
                               ("biome", "minecraft:unknown"), ("offset", -1),
                               ("offset", 0.0), ("axes", [[0, 0]] * 5),
                               ("axes", [[1, 0]] * 6), ("axes", [[-20001, 0]] * 6)):
            value = copy.deepcopy(parameters)
            value["minecraft:nether"][0][field] = changed
            with self.assertRaises(RuntimeError):
                oracle.verify_rows(value, keys)
        value = copy.deepcopy(parameters)
        value["minecraft:nether"].pop()
        with self.assertRaisesRegex(RuntimeError, "count differs"):
            oracle.verify_rows(value, keys)
        value = copy.deepcopy(parameters)
        value["minecraft:end"] = []
        with self.assertRaisesRegex(RuntimeError, "preset keys"):
            oracle.verify_rows(value, keys)


if __name__ == "__main__":
    unittest.main()
