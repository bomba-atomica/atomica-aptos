/// IBE Golden Vector Fixtures (Test-Only)
///
/// This module provides access to golden test vectors for IBE testing.
/// All values are encoded as hex strings matching the JSON golden vectors.
///
/// The golden vectors are generated from real PVSS transcripts and include:
/// - Identity computation test vectors
/// - IBE roundtrip test vectors with full encryption/decryption verification
///
/// # Usage
///
/// ```move
/// use aptos_framework::ibe_golden_vector_fixtures as fixtures;
///
/// // Get identity hash for a specific timelock
/// let identity = fixtures::get_identity_hash(0, 1000000000000);
/// assert!(identity == x"cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355", 0);
/// ```
module aptos_framework::ibe_golden_vector_fixtures {

    // ================================
    // Identity Computation Vectors
    // ================================

    /// Returns the identity hash for timelock_id=0, deadline_us=1000000000000
    /// Expected: cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355
    public fun identity_0_1000000000000(): vector<u8> {
        x"cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355"
    }

    /// Returns H(identity) in G1 for timelock_id=0, deadline_us=1000000000000
    /// Expected: 95502e8ee330870f5002c4cfc834467a507fcadaa49f9da04c539f322858284e627937fb234993f78dd5869b57419aa9
    public fun h_identity_0_1000000000000(): vector<u8> {
        x"95502e8ee330870f5002c4cfc834467a507fcadaa49f9da04c539f322858284e627937fb234993f78dd5869b57419aa9"
    }

    /// Returns the identity hash for timelock_id=1, deadline_us=1000000000000
    /// Expected: 1235cfe3eb61fd7c5aa850477532822bbef4d9ca30decc2ca4405ebb30d5cab1
    public fun identity_1_1000000000000(): vector<u8> {
        x"1235cfe3eb61fd7c5aa850477532822bbef4d9ca30decc2ca4405ebb30d5cab1"
    }

    /// Returns H(identity) in G1 for timelock_id=1, deadline_us=1000000000000
    /// Expected: aa5dffd79cb804f313d5c2e882183fd8477d3f9a6424cb56fa373e2c7ce0d0904383af53a5680538423d02ac61eaa210
    public fun h_identity_1_1000000000000(): vector<u8> {
        x"aa5dffd79cb804f313d5c2e882183fd8477d3f9a6424cb56fa373e2c7ce0d0904383af53a5680538423d02ac61eaa210"
    }

    /// Returns the identity hash for timelock_id=0, deadline_us=2000000000000
    /// Expected: ee623d0d70b0a085f0e91a197853c627a9cd7b24cdac79d2782e912e35ddefe6
    public fun identity_0_2000000000000(): vector<u8> {
        x"ee623d0d70b0a085f0e91a197853c627a9cd7b24cdac79d2782e912e35ddefe6"
    }

    /// Returns H(identity) in G1 for timelock_id=0, deadline_us=2000000000000
    /// Expected: b6ef562742ce6af3dcd6335b07bbbd38a36c51d8f13a470a6da39f916ffa41ad3651f454820244a8a38308ef99ec3521
    public fun h_identity_0_2000000000000(): vector<u8> {
        x"b6ef562742ce6af3dcd6335b07bbbd38a36c51d8f13a470a6da39f916ffa41ad3651f454820244a8a38308ef99ec3521"
    }

    // ================================
    // Roundtrip Vector 1: 5 validators, threshold 3, equal weights
    // ================================

    /// Vector 1: RNG seed
    public fun roundtrip_1_rng_seed(): u64 { 12345 }

    /// Vector 1: Master secret key (32 bytes, little-endian hex)
    public fun roundtrip_1_msk(): vector<u8> {
        x"3fea0be5546f8322489b371987d8097874e8caa1f5e54314f40ad3d56e4c2f6c"
    }

    /// Vector 1: Master public key (96 bytes, G2 compressed hex)
    public fun roundtrip_1_mpk(): vector<u8> {
        x"82fb16f4572daf7bd95e9c208eb29f4e43fd78df7f4a7129ef438b03b4da1ade521cd25b1eaeec7398bc8e74dd371b5213e87029507649495dc9cf02ace2ca040e79af59d9c380a1bfafa619e511452516e1719ae917265bc87a7b1236a5b0be"
    }

    /// Vector 1: Identity hash (32 bytes)
    public fun roundtrip_1_identity(): vector<u8> {
        x"e3fb053eedce65f62fcbbfd56fe9bbec0abe3bfcc3c5ec54a913bd50c51a5dfe"
    }

    /// Vector 1: H(identity) in G1 (48 bytes)
    public fun roundtrip_1_h_identity(): vector<u8> {
        x"8ee74013044887e31110186f2f90d556e8780998295c69821011707562e44bf9fa28d8c331fa41aaacec7f5f8368b41e"
    }

    /// Vector 1: Threshold
    public fun roundtrip_1_threshold(): u64 { 3 }

    /// Vector 1: Total weight
    public fun roundtrip_1_total_weight(): u64 { 5 }

    /// Vector 1: Validator indices [0, 1, 2, 3, 4]
    public fun roundtrip_1_validator_indices(): vector<u64> {
        vector[0, 1, 2, 3, 4]
    }

    /// Vector 1: Validator weights [1, 1, 1, 1, 1]
    public fun roundtrip_1_validator_weights(): vector<u64> {
        vector[1, 1, 1, 1, 1]
    }

    /// Vector 1: DK shares in G1 (48 bytes each, 5 shares)
    public fun roundtrip_1_dk_shares(): vector<vector<u8>> {
        vector[
            x"9722f3fe074ff0467af66bbb6564aaeec41ac369dbc55a520e1197e2cabd01489fac2b5260c049e9ed26fdd872391d2d",
            x"b0cc6092fc45df1b1318952c08dd2a877b5b19deb725b63cfc41c48e84da6e8f77600efe4d38aac33273a87775112a31",
            x"ac20a63c6b62ed15a3424d85af25eb006795eba2172996529afbb70d29d1a57066a37ee1c98dea843fa2c998cb6b3919",
            x"b42700b02c7997b35f244481afa657a601e915046aed0dee5dcde214e91efdecbf3fcc9d21c11907aac23a84c43927d2",
            x"891c64345686bd154a277a7540ed7c32ad7a0abe30a40586515bf07738e6f63f054b6a55d6d5a2288da37f15335618bf"
        ]
    }

    /// Vector 1: Reconstructed DK in G1 (48 bytes)
    public fun roundtrip_1_reconstructed_dk(): vector<u8> {
        x"a46066de7604d2d51825cd139d720523d5584654cda567f80c3779304d2dec8842b41a7feec0db2a202409f45492593c"
    }

    /// Vector 1: Plaintext
    public fun roundtrip_1_plaintext(): vector<u8> {
        x"48656c6c6f2049424520676f6c64656e20766563746f7220746573742077697468205056535321"
    }

    /// Vector 1: Ciphertext U component (96 bytes, G2)
    public fun roundtrip_1_ciphertext_u(): vector<u8> {
        x"b33b839deddaab78e2819a94b74aade57aab61ec3ce41eaa45775eedcc0ea6d51994701e4021e2517f8c3d522ee0c8fb020471a9582fc4e6f64428d833c4ec0eceabfddb11c3c3c11c1829191bc18ccc71ecf9628301bc38ecd0d1f71baa12b1"
    }

    /// Vector 1: Ciphertext V component
    public fun roundtrip_1_ciphertext_v(): vector<u8> {
        x"509a8a216f2d56cd243bb83b91b107bb9f495ec1ecb7c4a2d48b9a6b214f42c4136d341ffb8079"
    }

    // ================================
    // Roundtrip Vector 2: 4 validators, threshold 2, equal weights
    // ================================

    /// Vector 2: RNG seed
    public fun roundtrip_2_rng_seed(): u64 { 67890 }

    /// Vector 2: Master secret key
    public fun roundtrip_2_msk(): vector<u8> {
        x"f5473a7e34fdbb238794a9a78a62cbe20f5f445fc2d5a5e9d27f0ad59b102f71"
    }

    /// Vector 2: Master public key
    public fun roundtrip_2_mpk(): vector<u8> {
        x"874774c6f8fc9d169328bd85db98d06147d6cf84706f341f43377fa19934b0b423ce2ce1df95bf9b607a8ef54c16468007b34e34540de15866b0a89503c777c898b863ed1772bd1e0c47122d1c248eb19f670b5263bd99e096841804e4cbd983"
    }

    /// Vector 2: Identity hash
    public fun roundtrip_2_identity(): vector<u8> {
        x"fe35f39be89aa3d57c43d37f0a4ecb02908c7a82c877eaf5c6517224aa09c46c"
    }

    /// Vector 2: H(identity) in G1
    public fun roundtrip_2_h_identity(): vector<u8> {
        x"8a8a5697ed766fd6d60dceef333ccd432ee3a22e286f964a3c0ad7407eccb965454c0aedb04db714ebc0aed08916d021"
    }

    /// Vector 2: Threshold
    public fun roundtrip_2_threshold(): u64 { 2 }

    /// Vector 2: Total weight
    public fun roundtrip_2_total_weight(): u64 { 4 }

    /// Vector 2: Validator indices [0, 1, 2, 3]
    public fun roundtrip_2_validator_indices(): vector<u64> {
        vector[0, 1, 2, 3]
    }

    /// Vector 2: Validator weights [1, 1, 1, 1]
    public fun roundtrip_2_validator_weights(): vector<u64> {
        vector[1, 1, 1, 1]
    }

    /// Vector 2: DK shares in G1 (4 shares)
    public fun roundtrip_2_dk_shares(): vector<vector<u8>> {
        vector[
            x"b60d3ddf9b0a596918276b9016723540aab31e024153a3f5b214150022e34444c2a8c6a08f2bc68f289f715ffdb93192",
            x"864b4a9211b0eb77dab0df356f124a86f946ed9004de11e0c0ae1ac529301eef9adf443373f9ab0a33d393db38528337",
            x"8f6e359c3d6d94ba5420695da8b441f1268d0bbe616bad8820125730a60eb589ddbf94e33c425e0a7871ec3d1a2ef520",
            x"a6edf2ef32d2e25a10b1374dd1c758545036f743f556252ae3451a489d454414820e4d9e287965a2c4a318e3d409c6c0"
        ]
    }

    /// Vector 2: Reconstructed DK
    public fun roundtrip_2_reconstructed_dk(): vector<u8> {
        x"9266dbe7f2c3a699fa1767d30c56507f792ee751fe457c6958f04dd8e0b6333d71fa609664b2cfada489547761effeb1"
    }

    /// Vector 2: Plaintext
    public fun roundtrip_2_plaintext(): vector<u8> {
        x"5365636f6e6420746573742063617365207769746820342076616c696461746f7273"
    }

    /// Vector 2: Ciphertext U
    public fun roundtrip_2_ciphertext_u(): vector<u8> {
        x"8b6111ff088b03482f90b5bd55c3224c286964c1612749af366f9826ffacca8ffcef749a796097febd6d09c647a3ef6111ad5b6688b9c696f66c0489980f2cbd4a139518d606ddac4135db2a141bf54cb75ee8c83f6a897de0b4925933eac29e"
    }

    /// Vector 2: Ciphertext V
    public fun roundtrip_2_ciphertext_v(): vector<u8> {
        x"9b0eaa53e097b8ffd0978f238be9b256cea118c5e7221d74a00b3197fafcbdfef2c5"
    }

    // ================================
    // Roundtrip Vector 3: 3 validators, threshold 3, unequal weights [2,1,2]
    // ================================

    /// Vector 3: RNG seed
    public fun roundtrip_3_rng_seed(): u64 { 99999 }

    /// Vector 3: Master secret key
    public fun roundtrip_3_msk(): vector<u8> {
        x"54efc669c86a245a0b3e15b52e957a432474d664ca824513cc3e489e1cb14007"
    }

    /// Vector 3: Master public key
    public fun roundtrip_3_mpk(): vector<u8> {
        x"986e5dc6d6b494fa5a5e8e1e2f1c5ddb22a021fd24c2117e3be5f2f9a844c04aa02c4270c09c0a7044c0f7e9780316a4102870266aab226c49949f8c069ac4858c9d1f5edb8ca90815e219072fe884b0a03fea5494a3e505498d3b1128996cb4"
    }

    /// Vector 3: Identity hash
    public fun roundtrip_3_identity(): vector<u8> {
        x"de8e8b23a9e541393a1835326e2e07e51571d61dd42a0c9dd1c2dab16186986d"
    }

    /// Vector 3: H(identity) in G1
    public fun roundtrip_3_h_identity(): vector<u8> {
        x"81d2a7a38c3852f64b1faf838f55a55fd03df97d8012b82f04e9925098516ac6bf6d8eb3fae98147ef41f522422b2a0d"
    }

    /// Vector 3: Threshold
    public fun roundtrip_3_threshold(): u64 { 3 }

    /// Vector 3: Total weight
    public fun roundtrip_3_total_weight(): u64 { 5 }

    /// Vector 3: Validator indices [0, 1, 2]
    public fun roundtrip_3_validator_indices(): vector<u64> {
        vector[0, 1, 2]
    }

    /// Vector 3: Validator weights [2, 1, 2]
    public fun roundtrip_3_validator_weights(): vector<u64> {
        vector[2, 1, 2]
    }

    /// Vector 3: DK shares in G1 (3 shares)
    public fun roundtrip_3_dk_shares(): vector<vector<u8>> {
        vector[
            x"862b3a0396c945a81c83866c60cbe87968455e2b74f3213bd52164e0cded92ccd8f147111a23f2723960281d646a6bfd",
            x"8a96fb8faed4eedb44496140cf2a47c09a56eca0da3a61e8306d11554ca90ba06493ec1d4523cf39326451dbb67d4549",
            x"a87fb1ad947d3e962b21a184a763e91ada32fbf596bf6dafb5aabec78dc52766ed03d9a80ddd8122b79a4acf4f450918"
        ]
    }

    /// Vector 3: Reconstructed DK
    public fun roundtrip_3_reconstructed_dk(): vector<u8> {
        x"afbaa0adb5ab4ef5d1c72e1aab6497f1fabd61d8f24a7c642593978e4ff18a080c10228fa8a61a8ec9f5ad0e15de1a76"
    }

    /// Vector 3: Plaintext
    public fun roundtrip_3_plaintext(): vector<u8> {
        x"556e657175616c20776569676874732074657374205b322c312c325d"
    }

    /// Vector 3: Ciphertext U
    public fun roundtrip_3_ciphertext_u(): vector<u8> {
        x"94078b8f4d68ebfaa8f568badfd271f99013fecdc37c48a320b2e768c38ee61d510971efffd3d07b223bf00fd809ee6906a8a0029f44ba15597aa8c695088fb0c040a03f1ffae76513f0c200c386653e80dcab242fedf535c53867f2f1cef812"
    }

    /// Vector 3: Ciphertext V
    public fun roundtrip_3_ciphertext_v(): vector<u8> {
        x"c577135d44e15ef14761ae4d7857faece4a4c5287f57ae6f6969351b"
    }

    // ================================
    // Roundtrip Vector 4: 4 validators, threshold 3, unequal weights [2,3,2,1]
    // ================================

    /// Vector 4: RNG seed
    public fun roundtrip_4_rng_seed(): u64 { 11111 }

    /// Vector 4: Master secret key
    public fun roundtrip_4_msk(): vector<u8> {
        x"7f97d8fbbd275b2b59753888b49c4bb2cf8910cf6621959ff5c146d224a50214"
    }

    /// Vector 4: Master public key
    public fun roundtrip_4_mpk(): vector<u8> {
        x"b71acf0e8a5c46a29ed8803219fc90d9ea8aa6b92c511fadadfab7e86a8a53839fbd4fcf47be51fb04c05b54f3e721d20da1f2f9e2d5c0b3f641a3cf2873344fd05d2d6cb6ec48d7e6a379d974b0cdf23aa9175b336f36916ec7f9dc0823fde7"
    }

    /// Vector 4: Identity hash
    public fun roundtrip_4_identity(): vector<u8> {
        x"5758871061dc3844df56aa064d31f0bc171b005ea70e9b1f866d17308e7db2a1"
    }

    /// Vector 4: H(identity) in G1
    public fun roundtrip_4_h_identity(): vector<u8> {
        x"a62edbe7b9728a6569fc67d2d6558c2295eef25a83588ab008f58fb641dc59b613cbb967f602ee722641cf10123121d3"
    }

    /// Vector 4: Threshold
    public fun roundtrip_4_threshold(): u64 { 3 }

    /// Vector 4: Total weight
    public fun roundtrip_4_total_weight(): u64 { 8 }

    /// Vector 4: Validator indices [0, 1, 2, 3]
    public fun roundtrip_4_validator_indices(): vector<u64> {
        vector[0, 1, 2, 3]
    }

    /// Vector 4: Validator weights [2, 3, 2, 1]
    public fun roundtrip_4_validator_weights(): vector<u64> {
        vector[2, 3, 2, 1]
    }

    /// Vector 4: DK shares in G1 (4 shares)
    public fun roundtrip_4_dk_shares(): vector<vector<u8>> {
        vector[
            x"a02a2adabcf4c3ec4a4c40dfb1a6a34bd4efba443dea05867cd88a3b45238fcc48721a83e95f5821826f8b5207846a09",
            x"b3b01331a629cfda8d731263e34bac8c4d9c794754499ae4be6fcb145fa793c21acdf0f1784684d5de286adc1f1f9f0a",
            x"8ca474497c7d328b4b8899041d25c69946607c9cdadf6ff7ccd05e3bf2acd29d27a156ec753f5d263fb98ab6917a061d",
            x"b3119a32f5b4d0ac7fa283e13cb7e09e971778d889b02e281e74d5b56b2eb73f6a0ca0d30549717bad57f3e8102b1079"
        ]
    }

    /// Vector 4: Reconstructed DK
    public fun roundtrip_4_reconstructed_dk(): vector<u8> {
        x"af3b251c96136842613b45d09a222986330395b5699449526fca7405e8d6bb2743306b193cf10bfdbcfe8cdc468eaa8d"
    }

    /// Vector 4: Plaintext
    public fun roundtrip_4_plaintext(): vector<u8> {
        x"556e657175616c2077656967687473205b322c332c322c315d20342076616c696461746f7273"
    }

    /// Vector 4: Ciphertext U
    public fun roundtrip_4_ciphertext_u(): vector<u8> {
        x"b13212a23d6f148499fa94c3c0994ba0fe5886a5d82247de8c3dccea06b6bff8326e59ab3009aa1fe9cec17faa6896af0989d078c87f42399efbbd48ce44308558b835e10bb17337dea70fe8387a3b3275e38282bca87bbb95c0ca934a47f584"
    }

    /// Vector 4: Ciphertext V
    public fun roundtrip_4_ciphertext_v(): vector<u8> {
        x"11050ed8b7027dedc1c064b7d39a059e7ddfd029d0410fa4c7be068b78f162cada64b21315a4"
    }

    // ================================
    // Helper Functions
    // ================================

    /// Get the identity hash fixture by timelock_id and deadline_us
    /// Returns empty vector if not found
    public fun get_identity_hash(timelock_id: u64, deadline_us: u64): vector<u8> {
        if (timelock_id == 0 && deadline_us == 1000000000000) {
            identity_0_1000000000000()
        } else if (timelock_id == 1 && deadline_us == 1000000000000) {
            identity_1_1000000000000()
        } else if (timelock_id == 0 && deadline_us == 2000000000000) {
            identity_0_2000000000000()
        } else {
            vector[]
        }
    }

    /// Get roundtrip vector by index (1-4)
    /// Returns validator indices for the given roundtrip
    public fun get_roundtrip_validator_indices(index: u64): vector<u64> {
        if (index == 1) roundtrip_1_validator_indices()
        else if (index == 2) roundtrip_2_validator_indices()
        else if (index == 3) roundtrip_3_validator_indices()
        else if (index == 4) roundtrip_4_validator_indices()
        else vector[]
    }

    /// Get roundtrip vector by index (1-4)
    /// Returns validator weights for the given roundtrip
    public fun get_roundtrip_validator_weights(index: u64): vector<u64> {
        if (index == 1) roundtrip_1_validator_weights()
        else if (index == 2) roundtrip_2_validator_weights()
        else if (index == 3) roundtrip_3_validator_weights()
        else if (index == 4) roundtrip_4_validator_weights()
        else vector[]
    }

    /// Get roundtrip vector by index (1-4)
    /// Returns DK shares for the given roundtrip
    public fun get_roundtrip_dk_shares(index: u64): vector<vector<u8>> {
        if (index == 1) roundtrip_1_dk_shares()
        else if (index == 2) roundtrip_2_dk_shares()
        else if (index == 3) roundtrip_3_dk_shares()
        else if (index == 4) roundtrip_4_dk_shares()
        else vector[]
    }

    /// Get roundtrip vector by index (1-4)
    /// Returns identity for the given roundtrip
    public fun get_roundtrip_identity(index: u64): vector<u8> {
        if (index == 1) roundtrip_1_identity()
        else if (index == 2) roundtrip_2_identity()
        else if (index == 3) roundtrip_3_identity()
        else if (index == 4) roundtrip_4_identity()
        else vector[]
    }

    /// Get roundtrip vector by index (1-4)
    /// Returns total weight for the given roundtrip
    public fun get_roundtrip_total_weight(index: u64): u64 {
        if (index == 1) roundtrip_1_total_weight()
        else if (index == 2) roundtrip_2_total_weight()
        else if (index == 3) roundtrip_3_total_weight()
        else if (index == 4) roundtrip_4_total_weight()
        else 0
    }

    /// Get roundtrip vector by index (1-4)
    /// Returns threshold for the given roundtrip
    public fun get_roundtrip_threshold(index: u64): u64 {
        if (index == 1) roundtrip_1_threshold()
        else if (index == 2) roundtrip_2_threshold()
        else if (index == 3) roundtrip_3_threshold()
        else if (index == 4) roundtrip_4_threshold()
        else 0
    }

    /// Get roundtrip vector by index (1-4)
    /// Returns reconstructed DK for the given roundtrip
    public fun get_roundtrip_reconstructed_dk(index: u64): vector<u8> {
        if (index == 1) roundtrip_1_reconstructed_dk()
        else if (index == 2) roundtrip_2_reconstructed_dk()
        else if (index == 3) roundtrip_3_reconstructed_dk()
        else if (index == 4) roundtrip_4_reconstructed_dk()
        else vector[]
    }
}
