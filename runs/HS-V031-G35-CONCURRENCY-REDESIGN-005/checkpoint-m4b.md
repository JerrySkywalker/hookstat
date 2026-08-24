# M4B — complete-record single-write diagnostic

Status: CANDIDATE_SUPPORTED_FOR_REQUALIFICATION

The prior admitted p99 failure remained correlated with file-wide group sync.
Inspection found that each logically complete WAL record used six serialized
`write_all` calls. The candidate now constructs the unchanged HSWL header,
length, checksum, and one HSIP frame in memory, then performs one ordered
`write_all`. ACK still follows only successful completion of that full OS-buffer
append; WAL framing, ordering, recovery, and durability thresholds are unchanged.

Five corrected-collector diagnostics retained every observation. One client16
measurement returned the sanitized `read_ack_timeout` error. The other four all
passed the frozen budget:

| Run | client16 p50 ms | p95 ms | p99 ms |
| ---: | ---: | ---: | ---: |
| 2 | 0.1101 | 0.6919 | 1.0775 |
| 3 | 0.1145 | 0.4032 | 0.7757 |
| 4 | 0.1045 | 0.5528 | 1.3092 |
| 5 | 0.0791 | 0.3604 | 0.8215 |

All five stage diagnostics completed within budget. Queue-wait p95 was
0.0766–0.2248 ms, WAL-append p95 was 0.0043–0.0077 ms, and physical sync count
was 3–5 while all 25 logical threshold requests remained visible. No latency
shift appeared in WAL append or ACK write.

Focused framing, truncated-tail, checksum-corruption, and exact-threshold tests
pass. The full all-feature suite also passes, including the 16-client/10K and
100-client/100K IPC matrix. These diagnostics support requalification but do not
replace the paired-control five-run gate.

OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
