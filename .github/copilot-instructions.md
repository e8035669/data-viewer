# Copilot Instructions

## Build & Dev Commands

```sh
dx serve                          # dev server (web, default)
dx serve --platform desktop       # dev server (desktop/webview)
dx bundle --web --release         # production web bundle → target/dx/data-viewer/release/web/public
dx bundle --release --desktop     # production desktop AppImage
cargo clippy                      # lint (clippy.toml is configured)
```

Tailwind CSS is compiled automatically by `dx serve`/`dx bundle` from `tailwind.css` — no separate Tailwind step needed.

There is no test suite.

## Architecture

This is a **Dioxus 0.7** single-page app targeting both web (primary) and desktop. Use only Dioxus 0.7 APIs — `cx`, `Scope`, and `use_state` do not exist.

```
src/
  main.rs          # Route enum + App root + asset constants
  models.rs        # All API/domain types (Sensor, Device, Endpoint, Rule*, …)
  persistence.rs   # localStorage helpers via dioxus-sdk-storage
  views/           # Page-level components, one file per route
    global.rs      # Providers component — wraps entire app with shared context
  components/      # Reusable UI component library (shadcn-style)
    <name>/
      component.rs # Component logic
      style.css    # Component-scoped CSS loaded with asset!("./style.css")
      mod.rs       # Re-exports component.rs
```

### Global State (Context API)

`Providers` in `views/global.rs` wraps the router and provides three context values accessible anywhere in the tree:

| Type | How to consume |
|---|---|
| `Signal<Endpoints>` | `use_context::<Signal<Endpoints>>()` |
| `Signal<Projects>` | `use_context::<Signal<Projects>>()` |
| `HeaderContext` | `consume_context::<HeaderContext>()` |

Page components set the navbar title in a `use_effect`:
```rust
use_effect(|| {
    consume_context::<HeaderContext>().set_title("My Page");
});
```

### Endpoints

Two backend variants exist — `GeneralEndpoint` and `EdgeEndpoint` — both wrapped in the `Endpoint` enum and implementing `EndpointTrait`. URL construction always goes through the trait methods (`.all_device()`, `.sensor(device_id, sensor_id)`, etc.). Never construct API URLs by hand.

### Models

Many domain types come in pairs: a full read type (`Sensor`, `Device`) and an edit/create type (`EditSensor`, `EditDevice`) that omits `id` and skips `None` fields on serialization. When posting to the API, use the `Edit*` variant.

### `#[derive(Store)]` Pattern

Local component state that needs lens-style field access uses the `Store` derive macro with a `#[store]` impl block:

```rust
#[derive(Store, Default)]
struct MyState {
    name: String,
    is_open: bool,
}

#[store]
impl<Lens> Store<MyState, Lens> {
    fn open(&mut self) {
        self.name().clear();
        self.is_open().set(true);
    }
}
```

## Key Conventions

- **Component structure**: every reusable component lives in `src/components/<name>/`. Add `component.rs`, `style.css`, and a `mod.rs` that re-exports. Register the new module in `src/components/mod.rs`.
- **Attribute forwarding**: use `#[props(extends=GlobalAttributes)]` to allow callers to pass arbitrary HTML attributes. Merge with `merge_attributes` from `dioxus-primitives` when you need to combine a base attribute set with forwarded ones.
- **Clippy rule**: never hold a `GenerationalRef`, `GenerationalRefMut`, or `WriteLock` across an `await` point. Drop the borrow before `await` or use `.clone()` on the value first.
- **Icons**: use `dioxus-free-icons` with `fa_solid_icons` / `fa_regular_icons` feature sets.
- **Serde**: API fields use `camelCase` (`#[serde(rename_all = "camelCase")]`); enum variants use `lowercase` or `SCREAMING_SNAKE_CASE` depending on the backend contract — check each type before adding variants.
- **Assets**: reference assets with `asset!("/assets/…")` from the crate root, or `asset!("./style.css")` for component-local CSS.
- **New routes**: add a variant to the `Route` enum in `main.rs`, create a view file under `src/views/`, and re-export it from `src/views/mod.rs`.
