# HookStat v0.2 Reliability Intelligence

Status: implemented by `HS-V02-G05`. This document defines deterministic,
runtime-neutral interpretation over already-normalized `HookInvocation`
records. It does not authorize new runtime inspection, instrumentation, trust
changes, receipt fields, or persistence.

## Trend windows and comparison

HookStat projects `24h`, `7d`, `30d`, and `All` from one reference instant.
For a bounded window of width `W`, the current period is inclusive
`[now - W, now]` and the immediately preceding comparison period is half-open
`[now - 2W, now - W)`. This avoids overlap at the shared boundary. `All` has
no previous period and therefore never fabricates a delta.

Every period shows runs, execution-failure numerator, terminal-sample
denominator, and rate. A comparison is classified only when both periods have
at least five terminal samples and coverage is `complete` or a deterministic
synthetic fixture. A material change is at least five percentage points:

- increase: regression;
- decrease: improvement;
- otherwise: stable.

Partial, sync-only, best-effort, unknown, and not-admitted coverage produces
`coverage_limited`; missing predecessor produces `insufficient_history`; small
or zero terminal denominators produce `insufficient_samples`. These states are
visible rather than being rendered as healthy or stable.

## Risk score

The deterministic 0–100 prioritisation score is intentionally interpretable,
not a probability or health verdict:

```text
rate_component = failure_rate_percent
                 * sample_count / (sample_count + 9)
                 * coverage_multiplier
score = clamp(round(rate_component + recency + trend + impact), 0, 100)
```

Coverage multipliers are 100% (complete/synthetic fixture), 70% (sync-only),
65% (partial), 55% (best-effort), 35% (unknown), and 0% (not admitted).
Recent observed execution failures contribute 15/10/5/0 points at
24h/7d/30d/older boundaries. A proven regression contributes 15 points and a
proven improvement subtracts 5. Bounded hook-event impact contributes 15 for
stop/session-end/permission-request, 10 for lifecycle/tool hooks, and 5 for
the remaining supported events.

The report and TUI show score and sample confidence. Consequently a `1/1`
failure remains visible but cannot automatically outrank a mature, meaningful
failure solely because its percentage is 100%.

## Failure fingerprints

Clusters group only execution failures in the selected window into the bounded
taxonomy `exit_nonzero`, `timed_out`, `protocol_failure`, or
`execution_failed`. An unrecognised legacy fingerprint is reduced to
`execution_failed`; arbitrary text is neither retained in the cluster nor
rendered. The model never consumes stdout, stderr, prompts, payloads, raw
commands, paths, or credentials.

## Revision timeline

Revision comparisons use a stable handler key, never a display name. Rows are
ordered by `(occurred_at_unix_ms, source_key, source_record_id)`. The latest
revision is current; the preceding contiguous revision epoch is previous. Each
shown side retains its real run/failure/sample/rate values. If no preceding
epoch exists—or either epoch lacks sufficient terminal samples—the comparison
is explicitly insufficient. A later reuse of an older revision starts a new
epoch; it is not silently merged with an earlier one.

## Privacy and boundaries

Intelligence is a pure analytics/report projection over existing canonical
records. It is evidence-source-neutral and read-only. Its machine schema is
versioned independently, and its UI text remains localized through the shared
catalog. No scoring, clustering, or revision display changes ledger identity,
deduplication, instrumentation, trust, receipt format, or analytics terminal
status semantics.
