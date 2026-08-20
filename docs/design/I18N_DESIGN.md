# HookStat v0.2 Internationalization Design

Status: foundation design. Full application translation belongs to `HS-V02-G03-I18N`.

## Reference and scope

This design follows TabBeacon's proven Human presentation boundary:

- concrete `en-US` and `zh-CN` locales plus an `auto` preference;
- typed message keys rather than language branches in domain code;
- deterministic locale-source precedence;
- user-local interface preferences with side-effect-free reads and explicit atomic writes;
- Human-only localization; machine contracts remain stable;
- CJK/grapheme-aware display width and monochrome semantics.

HookStat should improve catalog organization by splitting the two catalogs behind the same typed interface. This is an internal layout choice, not a separate HookStat visual language.

## Supported locale model

```text
InterfaceLanguage = Auto | EnUs | ZhCn
ResolvedLocale    = EnUs | ZhCn
LocaleTag         = auto | en-US | zh-CN
```

Only `en-US` and `zh-CN` are concrete render targets in v0.2. Unsupported values do not partially match an unrelated locale.

## Locale resolution

Resolution order:

```text
explicit admitted --lang value
  -> HOOKSTAT_LANG if admitted
  -> user-local Interface preference
  -> operating-system locale
  -> en-US
```

Rules:

- `auto` means continue to the next source.
- Accept equivalent case/underscore spellings and ignore POSIX encoding suffixes only through one tested parser, matching TabBeacon's behavior.
- Unsupported CLI values are reported as argument errors; unsupported environment or OS values continue safely.
- A malformed preference file produces a safe diagnostic and fallback but is never rewritten merely by reading it.
- The resolver returns both locale and source for Settings/Diagnostics explainability.

## Catalog architecture

Target internal structure:

```text
src/tui/locale/
  mod.rs       Locale, LocaleSource, MessageKey, resolver, lookup contract
  en_us.rs     complete en-US catalog
  zh_cn.rs     complete zh-CN catalog
```

The concrete file spelling follows Rust module rules; the external locale tags remain exactly `en-US` and `zh-CN`.

Conceptual API:

```text
enum MessageKey { ... }

Catalog::text(locale, key) -> &'static str
Catalog::format(locale, key, TypedArguments) -> String
```

The implementation may use compile-time Rust tables/matches to avoid a large dependency. Required behavior:

- the key type is closed and compiler-visible;
- duplicate keys are impossible or rejected by tests;
- every `en-US` key exists;
- every `zh-CN` key either exists or follows the explicit per-key fallback rule;
- formatting placeholders are identical across catalogs;
- untranslated debug placeholders fail release tests.

## Key naming and ownership

Keys describe semantics, not English wording or widget location alone.

Representative groups:

```text
app.title
nav.overview
nav.hooks
nav.diagnostics
nav.settings
view.hook_detail.title
field.runtime
field.coverage
field.failure_rate
field.sample_count
status.healthy
status.degraded
status.failed
status.loading
state.empty.no_receipts
state.error.refresh_failed
shortcut.back
shortcut.refresh
shortcut.search
shortcut.filter
shortcut.quit
```

Application/domain code provides typed data. Catalog text owns:

- headings;
- Human field labels;
- status names and explanations;
- navigation and shortcut descriptions;
- empty/loading/error guidance;
- diagnostics explanations and safe next actions;
- Settings labels and local-only persistence explanation.

The following remain locale-neutral:

- CLI commands, subcommands, and flag names;
- JSON keys and report schema versions;
- stable plain-output keys and receipt fields;
- database schema and persisted enum spellings;
- runtime IDs, evidence IDs, diagnostic IDs, handler keys, and revisions;
- Hook event storage values;
- paths when a separately governed diagnostic explicitly permits a sanitized path value.

## No hardcoded UI strings

After G03, user-visible strings must not appear directly in:

- `tui/views/*`;
- shared UI components;
- key handling/footer construction;
- loading/empty/error state constructors;
- Human diagnostic presentation.

Exceptions are deliberately non-translated tokens:

- literal key glyphs such as `r`, `/`, `q`, `Esc`;
- product and runtime proper names such as `HookStat` and `Codex` when the catalog treats them as invariant data;
- stable machine IDs shown in Metadata style;
- punctuation supplied by a shared locale-aware formatter.

Domain methods such as `TimeWindow::label`, `Runtime::label`, and `HookEvent::label` must not be called by the localized TUI. Future code may retain them temporarily for v0.1 machine/Human compatibility, but the v0.2 UI maps typed values through locale keys.

## Formatting and interpolation

- Numbers are prepared as typed values; the first v0.2 implementation may retain locale-neutral ASCII digits and fixed decimal punctuation for reliability metrics so reports remain comparable.
- Failure rate and sample count are formatted together by one helper and never separated by translation.
- Durations and timestamps use bounded typed formatters; no locale string builds SQL or query parameters.
- Interpolation values are escaped/sanitized for terminal display and cannot inject control characters.
- User annotations remain literal user content and are not treated as translation keys.
- Event fallback display names are localized at render time from `HookEvent`.

## Runtime language switching

Settings holds accepted and draft interface preferences separately.

```text
accepted.language
draft.language
resolved_locale
dirty
conflict
```

Behavior:

1. Selecting `auto`, `en-US`, or `zh-CN` updates `draft.language`.
2. The next frame resolves and renders from the draft immediately, including navigation, current content, overlays, and footer.
3. No preference file is written until explicit Apply.
4. Revert restores the accepted preference and language.
5. Quit with a dirty draft requires explicit discard/cancel.
6. A concurrently changed preference baseline becomes an explicit conflict; stale Apply is refused.

Machine output does not change when the runtime TUI language changes.

## Persistent preference

Recommended file:

```text
<HookStat user data root>/interface.toml
```

The current HookStat user data root is `%LOCALAPPDATA%\HookStat` (falling back to `%APPDATA%`) on Windows and `$XDG_DATA_HOME/HookStat` or `$HOME/.local/share/HookStat` on Unix. G03 may move neutral path resolution out of the Codex adapter, but must preserve existing data-root behavior.

Initial schema:

```toml
[interface]
language = "auto"
color = "auto"
reduced_motion = false
theme = "default"
```

Persistence requirements:

- no repository-local preference file;
- absent-file reads are side-effect free;
- first explicit Apply may create the file and parent directory;
- atomic replacement;
- preserve unknown/future fields;
- refuse symbolic-link or ownership hazards under the selected platform policy;
- snapshot/compare before writing so a stale draft does not overwrite concurrent change;
- no relation to Codex `hooks.json`, trust state, instrumentation manifest, receipts, or ledger schema.

## Fallback behavior

Fallback locale is `en-US`.

- Unknown requested locale: continue resolution or report an invalid explicit CLI argument.
- Missing `zh-CN` key: use the exact `en-US` key and record a test-visible catalog defect.
- Missing `en-US` key: release gate failure; a bounded key placeholder may appear only in development.
- Formatting mismatch: do not render a partially interpolated string; use the safe English fallback and surface a development diagnostic.
- Malformed preference: use `auto` behavior without rewriting the user's file.

## Display width and terminal behavior

HookStat must adopt TabBeacon-compatible Unicode primitives:

- measure terminal display cells, not `.len()` or character count;
- truncate on grapheme boundaries;
- test combining marks, full-width CJK, and joined emoji;
- avoid broken borders at normal, narrow, and minimum sizes;
- make field-label padding locale aware;
- keep status words/glyphs when no color is available.

Adding `unicode-width` and `unicode-segmentation` is expected to be a small, justified dependency change in G00/G03; this design train adds no dependency.

## Catalog rollout

1. G00 establishes key/resolver/catalog interfaces and localizes the shared shell/state components with a representative subset.
2. G01 uses keys for all new Reliability Center views, even if `zh-CN` temporarily falls back under an explicit development gate.
3. G02 adds identity/source/category keys without localizing internal IDs or user literals.
4. G03 completes all `en-US`/`zh-CN` Human/TUI strings, runtime switching, persistence, and parity tests.
5. G04/G05 add diagnostics and intelligence keys through the same catalog boundary.
6. G06 rejects missing keys, debug placeholders, broken CJK layouts, and locale-dependent machine contracts.

## Required tests

- locale parsing and resolution precedence for every source;
- `auto` continues rather than resolving as a concrete locale;
- unsupported and malformed input behavior;
- catalog key and interpolation parity;
- live draft switch updates all visible regions;
- Revert/discard/apply semantics;
- absent preference read does not write;
- atomic write, unknown-field preservation, concurrent-change refusal, and exact restore as applicable;
- `en-US` and `zh-CN` buffers for every view/state at normal/narrow/minimum widths;
- grapheme-safe truncation and CJK alignment;
- no-color/monochrome semantic equivalence;
- JSON, receipt, ledger, and stable plain keys remain locale independent;
- no user-visible hardcoded strings in localized view modules.

## Design disposition

```text
LOCALES=en-US,zh-CN
AUTO_LANGUAGE=true
TYPED_TRANSLATION_KEYS=true
RUNTIME_SWITCH=true
PERSISTENT_PREFERENCE=true
FALLBACK_LOCALE=en-US
MACHINE_CONTRACT_LOCALIZED=false
FULL_TRANSLATION_STARTED=false
```
