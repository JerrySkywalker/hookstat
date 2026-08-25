# Retained G36 corrected-method debug receipts

These two complete five-by-100 series exercised the corrected warm-up and
signed paired subtraction implementation, but were invoked through the debug
test profile. They are retained for RCA only and are not acceptance evidence.
Neither operation touched owner-live configuration or captured raw private
content.

| Receipt sequence | Cooperative worst p95 / p99 (ms) | Gaps | Warm worst p95 / p99 (ms) | Cold worst p95 (ms) | Healthy HookStat-induced timeouts | Classification |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `001` (before the OS-backed child wait) | 0.1925 / 0.2693 | 0 | 39.5605 / 117.2534 | 32.7291 | 0 | `INVALIDATED_BY_BUILD_PROFILE` |
| `002` (connection-reuse plus OS-backed child wait) | 0.1913 / 0.2983 | 0 | 32.7607 / 53.0405 | 34.4153 | 0 | `INVALIDATED_BY_BUILD_PROFILE` |

Receipt `002` series-level values, retained to make the classification
auditable, were:

```text
cooperative:
  p50: 0.0397, 0.0360, 0.0895, 0.0803, 0.0456
  p95: 0.1667, 0.1496, 0.1913, 0.1912, 0.1352
  p99: 0.2185, 0.2840, 0.2983, 0.2625, 0.2169
warm paired incremental:
  p50: 20.5924, 20.8339, 20.7571, 21.8685, 21.1064
  p95: 32.7607, 27.2599, 27.0046, 28.6172, 29.8983
  p99: 53.0405, 32.7972, 28.2122, 30.7429, 37.8338
cold paired incremental:
  p50: 23.8383, 17.6494, 19.2957, 20.5377, 22.2972
  p95: 31.0082, 25.2210, 27.2964, 27.7118, 34.4153
  p99: 44.0248, 29.2534, 33.9705, 46.1685, 58.1999
```

The qualification test now rejects a debug build before it writes an
acceptance-shaped receipt; a later release receipt uses the same corrected
measurement design.
