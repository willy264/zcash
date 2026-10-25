# Darkroom shielded fixtures

These are historical, real V4 Sapling transaction bytes from
[zcash-hackworks/darksidewalletd-test-data](https://github.com/zcash-hackworks/darksidewalletd-test-data)
at commit `4aa4e669a5f03aa954e6720817f52eaab2226708`, under the MIT license
reproduced in `UPSTREAM-LICENSE`.

`wallet.json` contains an **intentionally public upstream test spending key**.
It is only used to derive a local incoming viewing key. Never send real funds
to this key or its addresses. Its provenance is the upstream
[`testnet-canopy/README.md`](https://github.com/zcash-hackworks/darksidewalletd-test-data/blob/4aa4e669a5f03aa954e6720817f52eaab2226708/testnet-canopy/README.md).

The subject reads only this key and a public, fixed block range. It does not
read `ground_truth.json`, the payment's transaction ID, or the expected memo
when deciding which compact output belongs to it. Ownership is discovered by
real Sapling trial decryption. The offline fixture gate independently decrypts
compact and full output ciphertexts and checks their values agree.

The selected payment is `61088726…3649`: its output 0 decrypts to **70,000,000
zatoshis** and the original memo `To Han`, followed by a newline, a `from`
address, and a trailing U+0001 byte. `ground_truth.json` records that exact
plaintext. The two cover transactions (`8f064d23…c99a` and `15a677b6…0590`)
do not decrypt under the subject's incoming viewing key. All three contain
one Sapling spend, two Sapling outputs, and no transparent inputs or outputs.
`provenance.json` records immutable source URLs and SHA-256 digests.
The stager reads the canonical `payment.hex`; its txid-named upstream copy is
retained in `transactions/`. The gate requires their decoded transaction bytes
to be identical before accepting the staged payment. Hex artifacts use LF
line endings so their recorded file hashes are portable.

Darksidewalletd serves these historical transaction bytes in synthetic blocks.
This is a receive-only test chain, not a consensus-validated mainnet or testnet
deployment. The fixture gate checks transaction IDs, parseability, shielding
structure, and actual decryption; it does not verify zero-knowledge proofs,
signatures, expiry heights, or historical spend anchors. No fabricated proofs
or fabricated ciphertext are created by this project.

V4 is intentional: upstream darksidewalletd still uses SHA256d for staged
transaction IDs, which is wrong for V5 ZIP-244 transaction IDs
([upstream issue #456](https://github.com/zcash/lightwalletd/issues/456)).

The fixed window starts at testnet height **1,028,500**, the actual Canopy
activation in `zcash_protocol`. The upstream fixture README's 1,028,400 label
is not used as a consensus parameter. This window is inside the ZIP-212 grace
period, so both pre-Canopy cover ciphertext and the payment ciphertext can be
trial-decrypted under the correct protocol rules.

Compact blocks omit memo ciphertext. `subject-wallet --memo` therefore fetches
every distinct transaction in the fixed public scan window once, in public
block order, regardless of note ownership. This is a bounded demonstration
policy with bandwidth costs, not a general production wallet privacy solution.
The intentionally leaky mode makes an additional ownership-dependent lookup
immediately after recognizing its note. The default, without `--memo`, performs
only compact scanning in good mode.

The merchant's `reply_addr` demonstration field is the actual decrypted note's
recipient address. It is not an authenticated sender or refund address.

Run the offline gate with:

```sh
cargo run --locked -p subject-wallet -- --verify-fixture --output .runs/fixture.json
```
