# HS-G44 — Safe Hook Management

## Objective

Add Hook management actions only where HookStat can prove an official, bounded Codex mutation surface suitable for an external client.

Information/read parity is mandatory for v0.4. Write parity is conditional on upstream capability and must never be simulated by unsafe filesystem guessing.

## Preconditions

```text
G41=PASS
G42=PASS
G43=PASS
```

## Capability qualification first

Audit the pinned Codex source/protocol for externally usable operations corresponding to current `/hooks` actions, including:

```text
enable / disable selected hook
trust selected hook
trust all review-needed hooks
managed hook behavior
```

Record exact method/config route, identity fields, preconditions, response/verification semantics, and version pin.

Do not infer that an internal Codex TUI `AppEvent` is automatically an external protocol contract.

## Admission outcomes

Allowed:

```text
WRITE_PARITY=PASS
WRITE_PARITY=UPSTREAM_UNAVAILABLE
```

If no stable safe route is proven, v0.4 remains read-complete and displays why mutation is unavailable.

## Mutation safety

Any admitted action must:

- target an exact current runtime handler identity;
- honor current hash/review preconditions where applicable;
- refuse stale catalog state;
- never mutate managed hooks;
- never grant trust implicitly when merely enabling;
- use only the official runtime route;
- refresh `hooks/list` after mutation and require confirmed resulting state;
- fail without rewriting unrelated configuration;
- preserve HookStat's no-trust-bypass contract.

## UI behavior

Actions must be discoverable but subordinate to current state:

```text
Managed       -> read-only explanation
Needs review  -> trust action if admitted
Trusted       -> enable/disable if admitted
Unsupported   -> read-only + reason
```

Do not optimistically display a mutated state before runtime confirmation unless the official protocol contract and final verification make this safe.

## No direct config imitation

Forbidden merely to match `/hooks` UX:

```text
plugin config filesystem guessing
managed config rewriting
trust hash fabrication
unscoped config edits
HookStat launcher interception
```

Existing explicit HookStat instrumentation apply/trust logic remains separately governed and must not be confused with generic current-hook management.

## Tests

If writes are admitted, cover:

- enable -> confirmed enabled;
- disable -> confirmed disabled;
- stale identity/hash refusal;
- needs-review refusal until trust where required;
- trust selected;
- trust all only exact eligible set;
- managed read-only;
- unrelated hook state unchanged;
- runtime error rollback/non-claim;
- post-write catalog refresh;
- version incompatibility -> unavailable.

## Acceptance

One of:

```text
READ_PARITY=PASS
WRITE_PARITY=PASS
```

or truthful:

```text
READ_PARITY=PASS
WRITE_PARITY=UPSTREAM_UNAVAILABLE
```

Always:

```text
MANAGED_HOOK_MUTATIONS=0
TRUST_BYPASS=false
UNOFFICIAL_CONFIG_GUESSING=false
POST_MUTATION_RUNTIME_VERIFICATION=true when writes admitted
CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

G45 owner A/B dogfood.
