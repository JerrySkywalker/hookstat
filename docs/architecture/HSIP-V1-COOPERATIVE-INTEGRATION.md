# HSIP v1 cooperative integration boundary

## Release decision

HookStat v0.3.1 admits cooperative local IPC as a production integration. It
does not publish `CooperativeProducer` as a stable Rust API and does not require
a cooperative consumer to depend on HookStat's CLI, TUI, analytics, SQLite, or
workbench implementation.

The narrow distribution model is:

```text
COOPERATIVE_DISTRIBUTION_MODEL=RUNTIME_INTEGRATION_OWNED_HSIP_V1_CLIENT
IPC_PROTOCOL_VERSION=1
FULL_HOOKSTAT_PRODUCT_DEPENDENCIES_REQUIRED=false
TABBEACON_COOPERATIVE_PATH_REMAINS_POSSIBLE=true
```

A consumer owns its small adapter and pins the versioned HSIP contract. It may
vendor the package-internal `src/ipc_client.rs` source at an audited HookStat
revision or independently implement the documented wire and local-transport
rules. Either choice is consumer build-time integration, not a third evidence
transport and not a public `hookstat` library-API commitment.

## Normative contract

The normative v1 envelope, field bounds, privacy exclusions, local endpoint,
acknowledgement, overload, and WAL-before-ACK rules are specified in
[`HS-G35-RUNTIME-NEUTRAL-IPC-BROKER.md`](HS-G35-RUNTIME-NEUTRAL-IPC-BROKER.md).
The wire identity is `HSIP`, protocol version is exactly `1`, the maximum frame
is 1024 bytes, and opaque references are at most 128 ASCII identifier bytes.

A cooperative adapter must:

- emit bounded START and COMPLETE frames without wrapping the observed Hook;
- treat broker unavailability, overload, rejection, contention, or deadline
  exhaustion as an observation gap rather than a Hook failure;
- never replay a frame after an uncertain write or acknowledgement;
- use only the owner-local Named Pipe on Windows or Unix Domain Socket on Unix;
- serialize no prompt, tool data, command, path, standard stream, credential,
  token, or other private content;
- pin HSIP version and reject incompatible protocol changes explicitly.

The installed `hookstat-ipc-broker` remains the receiving executable. Broker
startup is optional and bounded; no machine-global service or network listener
is introduced.

## TabBeacon proof

The retained controlled proof used TabBeacon
`b3f5685c37f1386f3edceb6d1d3a27403c59dddf` and the exact
`src/ipc_client.rs` SHA-256
`a6431eea9e2b373781d46b7f3a0694b5658ea8ca2b8076df6cd3a496ca6acdd6`.
It exercised the real G35 broker around the real `CodexHookRuntime`, required
no transparent wrapper, and excluded full HookStat product dependencies from
the adapter. The current G36 candidate retains that exact client source hash.

This proves the boundary remains possible; it does not publish TabBeacon,
modify Owner configuration, or make HookStat responsible for a consumer's
release.
