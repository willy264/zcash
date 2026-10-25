# Fixture stager

`stager --endpoint http://127.0.0.1:9067 --fixtures fixtures`

Reads `staging.json`, validates the local transaction files and height range, then
calls `Reset`, `StageBlocksCreate`, `StageTransactionsStream`, and `ApplyStaged`.
It waits for the served compact tip before succeeding. Transaction bytes are sent
directly over the admin stream; the backend never has to open a user-supplied URL.

Manifest shape:

```json
{
  "chain_name": "test",
  "branch_id": "e9ff75a6",
  "start_height": 1028500,
  "end_height": 1028510,
  "transactions": [{"file": "payment.hex", "height": 1028505}]
}
```

Synthetic block creation gives the replay a deterministic chain container.
Darkside does not verify the fixtures' consensus validity. Fixture provenance and
the subject's actual cryptographic decryption are checked separately.
