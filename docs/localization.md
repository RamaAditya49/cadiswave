# Localization

CadisWave uses application-owned Fluent bundles for English and Bahasa Indonesia.
Catalogs reside in `data/locales/` and are embedded during compilation.
Language preferences use stable `system`, `en`, and `id` values.
Unsupported system languages use English.
Language changes retain the runtime, routes, and device command ownership.

Use complete messages with named values.
Keep technical errors, device identifiers, and user names unchanged.
New device views use explicit message keys and translation methods.
Existing native views use bounded weak text bindings for authored messages.
Mark user text with `i18n::protect` before binding a widget tree.
Use explicit bindings for translated properties beside protected data.
Bindings release destroyed widgets and reject text changed by another owner.

Run `cargo test --locked -p cadiswave-core --test locale`.
Run `cargo test --locked -p cadiswave-desktop --lib i18n`.
Run ignored native binding tests through the isolated GTK runner.
Catalog tests check syntax, message keys, and named arguments.
