# Architecture

[Back to README](../README.md)

Runtime boundary
----------------

winedroid is not a VM, emulator, container wrapper, or hidden Android install.
The runtime must be owned by winedroid.

Out of scope:

- hardware or system virtual machines
- full Android system images as the source of truth
- container-first Android systems
- adb-installing into a separate Android OS
- depending on Android `system_server` as the runtime owner
- host-side shortcuts that bypass APK/framework behavior

In scope:

- parsing APKs, manifests, resources, DEX files, and native libraries
- executing DEX through ART or a winedroid-owned DEX runtime strategy
- Android framework class compatibility
- winedroid-owned package, activity, resource, lifecycle, and service models
- Android ELF loading with a Bionic-compatible runtime
- native CPU translation when APK ABI and host CPU differ
- host-native windows, input, storage, networking, clipboard, and permissions

The distinction matters: ART is a managed bytecode runtime, not a hardware or
system VM. Reusing ART is acceptable only if winedroid owns the surrounding
process model and services instead of booting Android.

Architecture laws
-----------------

These rules are intended to keep the codebase from collapsing into catch-all
modules.

- One module owns one responsibility.
- Refactors must move existing behavior first; behavior changes happen in later
  patches.
- Do not replace implemented paths with stubs during refactors.
- Unsupported Android behavior must be explicit. Best-effort execution logs
  missing calls and substitutes diagnostic return values at the call boundary;
  WINEDROID_STRICT=1 rejects them. Neither mode treats substitutes as implemented
  behavior. Runtime errors and guest exceptions still propagate.
- No app-specific rendering hacks. Compatibility work must become a generic
  Android/runtime primitive.
- Rendering reads UI state and draws pixels. It must not mutate runtime state.
- Host integration must go through narrow explicit bridges.
- Launch/session code wires components together. It must not become a runtime,
  renderer, window manager, or Android framework implementation.
- Avoid framework overengineering: no trait soup, plugin systems, or dynamic
  dispatch layers unless a concrete compatibility need forces them.
- Do not optimize runtime performance until behavior and module boundaries are
  stable.

Layer ownership
---------------

Target dependency direction:

```text
APK -> Dex/ClassLoader -> Runtime -> Android framework -> Services/UI -> Renderer/Host
```

Layer responsibilities:

- `apk`: APK archive access and package file inspection.
- `manifest`: Android manifest parsing and launchable component metadata.
- `resource_table`: compiled Android resource table parsing and lookup.
- `binary_xml`: Android binary XML decoding.
- `dex`: DEX file parsing, class/method/field metadata, and bytecode data.
- `dex_interpreter`: DEX instruction execution mechanics only.
- `runtime`: object state, heap, arrays, fields, class loading, method dispatch,
  and builtin routing.
- `android`: Android Java framework classes grouped by package/domain.
- `services`: emulated Android system services such as activity manager,
  package manager, resource manager, window manager, storage, input, and
  clipboard.
- `ui`: Android UI state, view trees, layout, hit testing, overlays, active
  surfaces, and input routing.
- `render`: pixels from UI state. Skia belongs here.
- `host`: OS integration: windows, event loops, browser opening, platform input,
  files, clipboard, notifications, and platform-specific glue.
- `native`: ABI selection, ELF loading, JNI, linker behavior, Bionic-compatible
  runtime, and CPU translation hooks.
- `launch`: app startup/session setup only.

Runtime contract
----------------

The target runtime model is Android-shaped. If code cannot preserve this model,
it should report a clear missing primitive. The call boundary may continue in
best-effort mode; strict validation must reject that missing behavior.

- Install produces reusable package artifacts: manifest identity, launch
  component, APK source metadata, DEX metadata, resources.arsc summary, resource
  file index, selected ABI, native inventory, and installed paths.
- Launch consumes installed package artifacts and starts an app process. It
  should not keep reparsing install-time package metadata as the source of
  truth.
- The VM owns class loading, loaded class records, dex caches, resolved
  method/field lookup, string/type/proto lookup, object storage, static fields,
  arrays, and method dispatch.
- The main thread owns deterministic task dispatch. ActivityThread lifecycle
  work, Handler posts, Choreographer callbacks, and ViewRoot traversal must flow
  through the main-thread queue instead of direct launch-time shortcuts.
- Activity lifecycle is scheduled as ActivityThread-style work. Direct calls are
  acceptable only inside the lifecycle task handler that owns the Android order.
- ViewRoot traversal is request-driven. `requestLayout` schedules measure,
  layout, and draw; `invalidate` schedules draw; no request means the last
  committed frame remains current.
- Rendering is retained-frame based. Framework/app code records runtime canvas
  frames or display-list-equivalent state, and the host presents the latest
  committed frame.
- Compose is normal app bytecode running on the VM plus Android framework
  primitives. Do not add Compose-specific rendering shortcuts or app-specific
  branches to make one APK look correct.
- Native execution is either a real JNI/ELF/Bionic strategy or explicitly
  unsupported until native code is reached. Do not silently treat native code as
  successful.

Current refactor pressure
-------------------------

The project is still early, but several files have grown beyond their intended
responsibility. Before adding deeper compatibility features, prefer extracting
clear boundaries from existing working code.

Primary extraction targets:

- `launch`: keep as orchestration; move worker plumbing and host projection out.
- `dex_run.rs`: split runtime state, builtins, dispatch, and Android framework
  behavior.
- `dex_run/view.rs`: split by Android concepts such as View, ViewGroup,
  TextView, Canvas, Dialog, Preference, and Compose/custom View support.
- `layout_render.rs`: split resource/layout loading, measurement, and render
  projection.
- `host_window.rs`: keep focused on host window and input integration.

Compatibility workflow
----------------------

New app support is trace-driven, generic, and grounded in the real Android
sources. The canonical workflow, coding rules, source-of-truth policy, and
testing harness live in [AGENTS.md](../AGENTS.md).

Optimization policy
-------------------

Do not optimize first. Correctness and boundaries matter more right now.

Allowed now:

- smaller modules
- simpler ownership
- clearer error paths
- fewer fallbacks
- lower coupling
- less duplicated projection logic

Deferred until the model is stable:

- runtime performance tuning
- renderer caching
- interpreter hot-path optimization
- speculative native execution shortcuts
- broad fallback systems

