# Prior Art: Drand Timelock (`tlock`)

**Source:** [https://github.com/drand/tlock](https://github.com/drand/tlock)

The `tlock` library by Drand is the reference implementation for practical timelock encryption using a threshold network. It combines **Identity-Based Encryption (IBE)** with the **age** encryption format to create a user-friendly CLI and library.

## 1. Protocol Architecture

The system implements a **Hybrid Encryption** scheme:

1.  **Symmetric Layer**: The payload is encrypted using `age` (ChaCha20-Poly1305).
2.  **KEM Layer**: The symmetric key (File Key) is encapsulated using Boneh-Franklin IBE.
3.  **Trust Anchor**: The Drand network serves as the Private Key Generator (PKG), but keys are generated automatically via the beacon protocol rather than on-demand.

### Step-by-Step Protocol

#### Encryption (Client-Side)

1.  **Setup**: The client initializes with the Drand Network's **Master Public Key (MPK)** and a target **Round Number** (representing the future time).
2.  **Hybrid Encryption**: The client uses the `age` library to generate a random ephemeral **File Key** (DEK). The actual data payload is encrypted with this File Key.
3.  **Identity Derivation**: The client derives an IBE Identity from the target round number:
    ```
    ID = Hash(RoundNumber)
    ```
4.  **Key Encapsulation**: The client encrypts the **File Key** using the **Boneh-Franklin IBE** scheme:
    - **Inputs**: Drand Master Public Key, Derived Identity ($ID$), File Key.
    - **Output**: Ciphertext tuple $(U, V, W)$.
5.  **Packaging**: The encrypted File Key is stored in an `age` header stanza of type `tlock`, along with the target round number and chain hash.

#### Decryption (Client-Side)

1.  **Header Parsing**: The client reads the `age` header to find the `tlock` stanza and extracts the target **Round Number**.
2.  **Key Fetching**: The client queries the Drand network (via HTTP/P2P) for the **Beacon Signature** corresponding to that specific Round Number.
    - _Note: This signature is only available after the time for that round has passed._
3.  **IBE Decryption**: The client uses the fetched **Beacon Signature** as the **IBE Private Key** to decrypt the File Key.
    - **Inputs**: Ciphertext $(U, V, W)$, Beacon Signature (acting as Private Key).
    - **Output**: The symmetric File Key.
4.  **Payload Decryption**: The client uses the File Key to decrypt the actual data payload.

---

## 2. Key Material

The system relies on the **BLS12-381** pairing-friendly elliptic curve. A key innovation is the dual-use of BLS signatures as IBE private keys.

| Component             | Cryptographic Object         | Description                                                                 |
| :-------------------- | :--------------------------- | :-------------------------------------------------------------------------- |
| **Master Public Key** | $P_{pub} \in G_1$ (or $G_2$) | The distributed public key of the Drand network. Available to everyone.     |
| **Master Secret Key** | $s \in \mathbb{F}_q$         | The distributed secret share held by Drand nodes. **Never reconstructed.**  |
| **Identity**          | $Q_{id} \in G_2$ (or $G_1$)  | $H(\text{RoundNumber})$. A point on the curve derived from the time.        |
| **Decryption Key**    | $d_{id} \in G_2$ (or $G_1$)  | The Drand **BLS Signature** on the round number. $d_{id} = s \cdot Q_{id}$. |
| **File Key**          | Symmetric Key                | Ephemeral key used by `age` to encrypt the actual content.                  |

_Note: Drand supports both "Unchained" (G2 signatures) and "Short Signature" (G1 signatures) schemes. `tlock` handles both by swapping the groups for keys and signatures._

---

## 3. Distributed & Cooperative Key Creation

The "Decryption Key" for a specific time (Round $N$) is simply the **Drand Beacon Signature** for Round $N$. This key is created cooperatively by the Drand validator nodes using **Threshold Cryptography**, ensuring no single party can create it early.

### The Process

1.  **Setup (DKG)**
    - When the network launches, nodes run a **Distributed Key Generation** (Pedersen DKG).
    - They generate a shared Master Secret Key $s$.
    - **Crucially**: No single node ever knows $s$. Each node $i$ only holds a partial share $s_i$.
    - They compute and publish the Master Public Key $P_{pub}$.

2.  **Key Generation (Beacon Pulse)**
    - When the time for Round $N$ arrives, each node $i$ signs the message $m = H(N)$ using its partial secret share $s_i$.
    - Partial Signature: $\sigma_i = s_i \cdot H(N)$.
    - Nodes broadcast these partial signatures to the network.

3.  **Aggregation (Key Revelation)**
    - Once a threshold number of partial signatures ($t$ out of $n$) are collected, any node can aggregate them using Lagrange interpolation.
    - **Result**: The final group signature $\sigma = s \cdot H(N)$.
    - This signature $\sigma$ serves two purposes:
      1.  It is the randomness beacon output.
      2.  **It is the IBE Private Decryption Key** for Round $N$.

4.  **Publication**
    - The signature is published to the network.
    - `tlock` clients download this signature and use it to decrypt their files.

### Security Guarantees

This design guarantees that the decryption key cannot exist mathematically until a threshold of honest nodes agrees that the timestamp for Round $N$ has arrived.
