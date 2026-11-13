# multi-log-macros

Proc macros that send one log statement to several loggers. The import path is
`multi_log_macros`.

```rust
use multi_log_macros::info;

info!("ntp: offset {:?}", offset);
```

expands (in the calling crate) to

```rust
{
    #[cfg(feature = "defmt")]
    ::defmt::info!("ntp: offset {:?}", offset);

    #[cfg(feature = "log")]
    ::log::info!("ntp: offset {:?}", offset);

    #[cfg(not(any(feature = "defmt", feature = "log")))]
    let _ = ::core::format_args!("ntp: offset {:?}", offset);

    #[cfg(feature = "log-extra")]
    {
        crate::mqtt_log::info!("ntp: offset {:?}", offset);
    }
}
```

## What the calling crate has to declare

* `defmt` and/or `log` features to select a back end. Both may be enabled; with
  neither, the statement expands to nothing (or, when `multi-log-macros` is
  built with the `consume-unused` feature, to `let _ = format_args!(…)` so the
  arguments are still used and no unused-variable warning appears).
* `log-extra` when it wants the extra sink. Without the feature the sink call is
  removed before name resolution, so the module does not have to exist.
* `LOG_EXTRA_PREFIX` in the build environment (`[env]` in `.cargo/config.toml`)
  naming the sink path, e.g. `crate::mqtt_log`. It is read while the crate is
  compiled; add `cargo:rerun-if-env-changed=LOG_EXTRA_PREFIX` to the build
  script so a change re-expands the macros.

`LOG_EXTRA_PREFIX` must not start with `::` (`::crate` is not valid syntax).
`crate::mqtt_log` and `::some_crate::sink` are both accepted.

## The sink's contract

The module at the prefix has to provide macros `trace!`, `debug!`, `info!`,
`warn!` and `error!` that accept `core::fmt`-style arguments, like `log`'s do.
They are invoked with the same tokens as the statement, so the arguments must
implement `core::fmt::Display`/`Debug` (a type that only implements
`defmt::Format` cannot be rendered by the sink).

They must also be reachable by path: a `macro_rules!` macro in the module plus
`pub(crate) use trace;` (and so on) makes `crate::the_module::info!(…)` work.

## Why a proc macro

`defmt` derives the file and line of a record from the DWARF entry of the
format-string static it interns, and a `macro_rules!` wrapper would intern that
static in the wrapper's own file — every record would point there. A proc
macro's tokens carry the span of its invocation, so the record points at the
statement that logged it. `log` (>= 0.4.22) uses `Location::caller()`, and
`module_path!()` expands to the invocation's module, so both back ends and the
sink report the right module as well.
