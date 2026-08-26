# G36 idle-helper frontend floor

This nonpublishable, package-excluded crate is a bounded architecture
experiment. It does not implement a HookStat helper and is not an activation
path.

The server is a local namespaced endpoint owned by the launched user process.
It expires after a bounded idle interval or an exact connection count. The
fresh frontend parses only its endpoint, sends one fixed eight-byte probe,
receives one fixed eight-byte response, and exits. No capsule, handler command,
path, prompt, stream, credential, or HookStat evidence frame crosses the
prototype channel.

The benchmark measures:

- a persistent-client connect/write/acknowledgement control exchange; and
- the cache-warmed fresh frontend process lifetime including one control
  exchange.

This is a strict lower-bound diagnostic. It does not prove capsule/HMAC
ownership, handler execution, timeout or exit preservation, Job Object
containment, frontend/helper death semantics, cold startup, concurrency, or
G35 evidence behavior. A failed floor eliminates the helper option for the
observed environment; a passed floor would only justify a deeper prototype.
