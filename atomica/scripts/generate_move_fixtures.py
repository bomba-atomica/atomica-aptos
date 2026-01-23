#!/usr/bin/env python3
"""
Generate Move fixture code from IBE golden vectors JSON.

Usage: python3 generate_move_fixtures.py <path-to-ibe_golden_vectors.json>
"""

import json
import sys
from pathlib import Path

def format_hex_array(hex_strings):
    """Format a list of hex strings as Move vector literal."""
    lines = []
    for i, hex_str in enumerate(hex_strings):
        prefix = "            " if i > 0 else "        "
        suffix = "," if i < len(hex_strings) - 1 else ""
        lines.append(f'{prefix}x"{hex_str}"{suffix}')
    return "\n".join(lines)

def generate_fixtures(json_path):
    with open(json_path) as f:
        data = json.load(f)

    roundtrips = data["ibe_roundtrip_vectors"]

    output = []
    output.append('// Copyright © Aptos Foundation')
    output.append('// SPDX-License-Identifier: Apache-2.0')
    output.append('')
    output.append('//! IBE Golden Vector Fixtures (Auto-generated from golden_vectors.json)')
    output.append('//! Run: python3 generate_move_fixtures.py ../atomica/golden_vectors/ibe_golden_vectors.json')
    output.append('')
    output.append('module aptos_framework::ibe_golden_vector_fixtures {')
    output.append('    use std::vector;')
    output.append('')
    output.append('    // ====================================')
    output.append('    // Identity Fixtures')
    output.append('    // ====================================')
    output.append('')
    output.append('    public fun identity_0_1000000000000(): vector<u8> {')
    output.append('        x"cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355"')
    output.append('    }')
    output.append('')
    output.append('    // ====================================')
    output.append('    // Roundtrip 1: 5 validators, equal weights')
    output.append('    // ====================================')
    output.append('')

    rt1 = roundtrips[0]
    output.append(f'    public fun roundtrip_1_msk(): vector<u8> {{ x"{rt1["msk_hex"]}" }}')
    output.append(f'    public fun roundtrip_1_mpk_g2(): vector<u8> {{ x"{rt1["mpk_g2_hex"]}" }}')
    output.append(f'    public fun roundtrip_1_identity(): vector<u8> {{ x"{rt1["identity_hash_hex"]}" }}')
    output.append(f'    public fun roundtrip_1_h_identity_g1(): vector<u8> {{ x"{rt1["h_identity_g1_hex"]}" }}')
    output.append(f'    public fun roundtrip_1_threshold(): u64 {{ {rt1["threshold"]} }}')
    output.append(f'    public fun roundtrip_1_total_weight(): u64 {{ {rt1["total_weight"]} }}')
    output.append('    public fun roundtrip_1_validator_indices(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(i) for i in rt1["validator_indices"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_1_validator_weights(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(w) for w in rt1["validator_weights"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_1_dk_shares(): vector<vector<vector<u8>>> {')
    output.append('        let v = vector::empty<vector<vector<u8>>>();')
    for i, validator_shares in enumerate(rt1["dk_shares_g1_hex"]):
        output.append(f'        vector::push_back(&mut v, vector[')
        for j, share in enumerate(validator_shares):
            suffix = ',' if j < len(validator_shares) - 1 else ''
            output.append(f'            x"{share}"{suffix}')
        output.append('        ]);')
    output.append('        v')
    output.append('    }')
    output.append(f'    public fun roundtrip_1_reconstructed_dk(): vector<u8> {{ x"{rt1["reconstructed_dk_g1_hex"]}" }}')
    output.append(f'    public fun roundtrip_1_plaintext(): vector<u8> {{ x"{rt1["plaintext_hex"]}" }}')
    output.append('')
    output.append('    // ====================================')
    output.append('    // Roundtrip 2: 4 validators, threshold 2')
    output.append('    // ====================================')
    output.append('')

    rt2 = roundtrips[1]
    output.append(f'    public fun roundtrip_2_identity(): vector<u8> {{ x"{rt2["identity_hash_hex"]}" }}')
    output.append(f'    public fun roundtrip_2_threshold(): u64 {{ {rt2["threshold"]} }}')
    output.append(f'    public fun roundtrip_2_total_weight(): u64 {{ {rt2["total_weight"]} }}')
    output.append('    public fun roundtrip_2_validator_indices(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(i) for i in rt2["validator_indices"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_2_validator_weights(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(w) for w in rt2["validator_weights"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_2_dk_shares(): vector<vector<vector<u8>>> {')
    output.append('        let v = vector::empty<vector<vector<u8>>>();')
    for i, validator_shares in enumerate(rt2["dk_shares_g1_hex"]):
        output.append(f'        vector::push_back(&mut v, vector[')
        for j, share in enumerate(validator_shares):
            suffix = ',' if j < len(validator_shares) - 1 else ''
            output.append(f'            x"{share}"{suffix}')
        output.append('        ]);')
    output.append('        v')
    output.append('    }')
    output.append(f'    public fun roundtrip_2_reconstructed_dk(): vector<u8> {{ x"{rt2["reconstructed_dk_g1_hex"]}" }}')
    output.append('')
    output.append('    // ====================================')
    output.append('    // Roundtrip 3: 3 validators, threshold 3, unequal weights [2,1,2]')
    output.append('    // ====================================')
    output.append('')

    rt3 = roundtrips[2]
    output.append(f'    public fun roundtrip_3_identity(): vector<u8> {{ x"{rt3["identity_hash_hex"]}" }}')
    output.append(f'    public fun roundtrip_3_threshold(): u64 {{ {rt3["threshold"]} }}')
    output.append(f'    public fun roundtrip_3_total_weight(): u64 {{ {rt3["total_weight"]} }}')
    output.append('    public fun roundtrip_3_validator_indices(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(i) for i in rt3["validator_indices"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_3_validator_weights(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(w) for w in rt3["validator_weights"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_3_dk_shares(): vector<vector<vector<u8>>> {')
    output.append('        let v = vector::empty<vector<vector<u8>>>();')
    for i, validator_shares in enumerate(rt3["dk_shares_g1_hex"]):
        output.append(f'        vector::push_back(&mut v, vector[')
        for j, share in enumerate(validator_shares):
            suffix = ',' if j < len(validator_shares) - 1 else ''
            output.append(f'            x"{share}"{suffix}')
        output.append('        ]);')
    output.append('        v')
    output.append('    }')
    output.append(f'    public fun roundtrip_3_reconstructed_dk(): vector<u8> {{ x"{rt3["reconstructed_dk_g1_hex"]}" }}')
    output.append('')
    output.append('    // ====================================')
    output.append('    // Roundtrip 4: 4 validators, threshold 3, unequal weights [2,3,2,1]')
    output.append('    // ====================================')
    output.append('')

    rt4 = roundtrips[3]
    output.append(f'    public fun roundtrip_4_identity(): vector<u8> {{ x"{rt4["identity_hash_hex"]}" }}')
    output.append(f'    public fun roundtrip_4_threshold(): u64 {{ {rt4["threshold"]} }}')
    output.append(f'    public fun roundtrip_4_total_weight(): u64 {{ {rt4["total_weight"]} }}')
    output.append('    public fun roundtrip_4_validator_indices(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(i) for i in rt4["validator_indices"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_4_validator_weights(): vector<u64> {')
    output.append(f'        vector[{", ".join(str(w) for w in rt4["validator_weights"])}]')
    output.append('    }')
    output.append('    public fun roundtrip_4_dk_shares(): vector<vector<vector<u8>>> {')
    output.append('        let v = vector::empty<vector<vector<u8>>>();')
    for i, validator_shares in enumerate(rt4["dk_shares_g1_hex"]):
        output.append(f'        vector::push_back(&mut v, vector[')
        for j, share in enumerate(validator_shares):
            suffix = ',' if j < len(validator_shares) - 1 else ''
            output.append(f'            x"{share}"{suffix}')
        output.append('        ]);')
    output.append('        v')
    output.append('    }')
    output.append(f'    public fun roundtrip_4_reconstructed_dk(): vector<u8> {{ x"{rt4["reconstructed_dk_g1_hex"]}" }}')
    output.append('')
    output.append('    // ================================  ')
    output.append('    // Helper Functions')
    output.append('    // ================================')
    output.append('')
    output.append('    public fun get_roundtrip_validator_indices(index: u64): vector<u64> {')
    output.append('        if (index == 1) roundtrip_1_validator_indices()')
    output.append('        else if (index == 2) roundtrip_2_validator_indices()')
    output.append('        else if (index == 3) roundtrip_3_validator_indices()')
    output.append('        else if (index == 4) roundtrip_4_validator_indices()')
    output.append('        else vector[]')
    output.append('    }')
    output.append('')
    output.append('    public fun get_roundtrip_validator_weights(index: u64): vector<u64> {')
    output.append('        if (index == 1) roundtrip_1_validator_weights()')
    output.append('        else if (index == 2) roundtrip_2_validator_weights()')
    output.append('        else if (index == 3) roundtrip_3_validator_weights()')
    output.append('        else if (index == 4) roundtrip_4_validator_weights()')
    output.append('        else vector[]')
    output.append('    }')
    output.append('')
    output.append('    public fun get_roundtrip_dk_shares(index: u64): vector<vector<vector<u8>>> {')
    output.append('        if (index == 1) roundtrip_1_dk_shares()')
    output.append('        else if (index == 2) roundtrip_2_dk_shares()')
    output.append('        else if (index == 3) roundtrip_3_dk_shares()')
    output.append('        else if (index == 4) roundtrip_4_dk_shares()')
    output.append('        else vector::empty()')
    output.append('    }')
    output.append('')
    output.append('    public fun get_roundtrip_identity(index: u64): vector<u8> {')
    output.append('        if (index == 1) roundtrip_1_identity()')
    output.append('        else if (index == 2) roundtrip_2_identity()')
    output.append('        else if (index == 3) roundtrip_3_identity()')
    output.append('        else if (index == 4) roundtrip_4_identity()')
    output.append('        else vector[]')
    output.append('    }')
    output.append('')
    output.append('    public fun get_roundtrip_total_weight(index: u64): u64 {')
    output.append('        if (index == 1) roundtrip_1_total_weight()')
    output.append('        else if (index == 2) roundtrip_2_total_weight()')
    output.append('        else if (index == 3) roundtrip_3_total_weight()')
    output.append('        else if (index == 4) roundtrip_4_total_weight()')
    output.append('        else 0')
    output.append('    }')
    output.append('}')
    output.append('')

    return "\n".join(output)

if __name__ == "__main__":
    json_path = sys.argv[1] if len(sys.argv) > 1 else "atomica/golden_vectors/ibe_golden_vectors.json"
    output = generate_fixtures(json_path)
    print(output)
