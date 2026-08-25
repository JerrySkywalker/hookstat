# M4 — controlled diagnostic A/B

Status: CANDIDATE_SUPPORTED_NOT_ACCEPTED

Baseline: `f1de6fcb5b2f587b374e81a48aaf5c517e4fefe6`

Candidate: `c7f3ba944bda4594e403c40fc9bf42c4a07655f7`

Both trees were built in isolated release targets and measured with the
corrected per-thread collector and the same 16-client start barrier. These are
diagnostic observations, not paired-control qualification evidence.

| Tree | Run | client16 p50 ms | p95 ms | p99 ms | Frozen budget |
| --- | ---: | ---: | ---: | ---: | --- |
| baseline | 1 | 0.2066 | 0.9636 | 2.0618 | FAIL |
| baseline | 2 | 0.3317 | 1.3044 | 2.3015 | FAIL |
| baseline | 3 | 0.3157 | 1.5803 | 2.3686 | FAIL |
| candidate | 1 | 0.2009 | 0.6648 | 1.2440 | PASS |
| candidate | 2 | 0.1831 | 1.2285 | 2.3492 | FAIL diagnostic host observation |
| candidate | 3 | 0.3017 | 0.7440 | 1.3881 | PASS |

Stage timing materially improved. Baseline queue-wait p95 was
0.5849/0.8698/0.7277 ms; final candidate queue-wait p95 was
0.3962/0.5046/0.4507 ms. Median queue-wait p95 fell from 0.7277 ms to
0.4507 ms (38.1%). Physical syncs fell from 24 per measured burst to 6/8/8,
while all 25 logical durability threshold requests remained visible.

The improvement did not move latency into WAL append or broker ACK write:
candidate WAL-append p95 was 0.0144–0.0204 ms and ACK-write p95 was
0.0068–0.0083 ms. Because one uncontrolled diagnostic candidate observation
still exceeded the frozen budget, M4 does not claim acceptance. The next gate
is the pre-established paired-control qualification retaining every admitted
and rejected observation.

OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
