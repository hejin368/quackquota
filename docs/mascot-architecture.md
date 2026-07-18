# QuackQuota mascot architecture

This document reserves a future mascot design only. The current application has
no duck UI, animation assets, `duckName` setting, or runtime mascot renderer.

## Boundaries

- Mascot state is a presentation layer independent of Codex Provider, OAuth,
  App Server, refresh commands, and window lifecycle.
- The future state mapper consumes only stable `CodexQuotaSnapshot` data:
  lowest valid `remainingPercent`, the tightest window ID, reset transitions,
  and offline/invalid status.
- Missing, corrupt, or reduced-motion mascot assets must never prevent quota
  refresh, tray behavior, Settings, or Overlay rendering.

## Planned state model

`happy`, `normal`, `tired`, `anxious`, `critical`, `sleepy`, `celebrating`,
and `offline` remain the planned stable states. A pure state-mapping function
will own thresholds and reset-transition detection; it must compare stable
window IDs rather than array positions.

## Future lifecycle input

The current Windows lifecycle watcher exposes only a coarse local desktop-app
state (`running`, `not-running`, unavailable, or unsupported). A future mascot
mapper may consume that state together with quota data, but the watcher must
remain independent of mascot assets, names, animations, and interactions.
For example, `not-running` could select a later offline or resting visual; it
must not turn a quota refresh failure into a different provider state. No
mascot UI is implemented by this lifecycle foundation.

## Future local mascot profile

When a static mascot surface exists, it may introduce a separate local profile:

```ts
interface MascotProfile {
  name: string;
  assetPackId: string;
}
```

The future settings field is `duckName`:

- Type: `string`; trim leading and trailing whitespace.
- Accept Chinese, English, digits, and common symbols; allow 1–20 visible
  characters after normalization.
- Empty input restores a localized default name. That default copy is not fixed
  in this planning phase.
- Persist only in local application settings. It is not tied to a ChatGPT or
  OpenAI account, uploaded, synchronized, sent as telemetry, or written to
  ordinary logs.
- Mascot UI, animation state, and quota Provider remain decoupled. A future
  `MascotRenderer` reads `MascotProfile`; quota code does not read the name.

The name can later appear in the Overlay, status copy, and celebration text,
but no unused Settings input is added before a mascot surface exists.

## Assets and accessibility

- Use local PNG frame sequences or sprite sheets, normally 4–6 fps.
- Each asset pack needs a manifest with ID, display name, renderer, frame rate,
  states, fallback state, dimensions, and licensing metadata.
- Respect reduced motion by rendering a static frame and never bypass it for
  celebration states.
- Missing manifests, frames, or incompatible packs safely fall back to a
  static placeholder or no mascot; no remote asset downloads or executable
  asset code are allowed.
- Later packs may provide pixel-art or other skins while retaining the same
  `MascotState` semantics.

## Temporary icon and future replacement

The current blue code icon is a temporary development icon, not an OpenAI logo
and not the planned duck icon. Its single editable source is
`rust/icons/icon.png`; `rust/icons/icon.ico` is the derived Windows bundle.
The Tauri bundle, runtime windows, tray, and About view all consume that same
source. A future duck-icon change must replace this source and regenerate the
derived `.ico` through the existing icon workflow.
