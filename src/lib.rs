//! Log macros that dispatch one statement to several loggers.
//!
//! Each macro re-emits its input, unchanged, for every sink that the calling
//! crate has enabled:
//!
//! ```text
//! multi_log_macros::info!("x = {}", x)
//!   => ::defmt::info!("x = {}", x);    // #[cfg(feature = "defmt")]
//!   => ::log::info!("x = {}", x);      // #[cfg(feature = "log")]
//!   => <prefix>::info!("x = {}", x);   // #[cfg(feature = "log-extra")]
//! ```
//!
//! # Back ends
//!
//! The `defmt` and `log` back ends are selected by the *calling crate*: the
//! attributes are emitted into it, so `feature = "defmt"` and `feature = "log"`
//! mean what that crate declares them to mean. A consumer that uses these
//! macros therefore has to declare both features. A build with neither expands
//! to nothing unless this crate is built with the `consume-unused` feature,
//! which makes it evaluate the arguments through [`core::format_args`] instead,
//! so that values used only by a log statement are not reported as unused.
//!
//! The two back ends are independent: a crate that enables both gets both,
//! which is what a firmware that logs through `defmt` and mirrors the records
//! elsewhere wants. A crate that enables neither is not an error.
//!
//! # The extra sink
//!
//! `LOG_EXTRA_PREFIX` names a path whose `trace!`/`debug!`/`info!`/`warn!`/
//! `error!` macros take `core::fmt`-style arguments, exactly like `log`'s do.
//! When the variable is set while the calling crate is compiled, each
//! statement also emits `<prefix>::info!(…)`. The call is behind
//! `#[cfg(feature = "log-extra")]`, so a build of the crate that does not
//! declare and enable that feature drops the tokens before name resolution:
//! the sink module does not have to exist and none of its code is compiled.
//!
//! The value must be a valid path and must not start with `::` (`::crate` is
//! not valid syntax). `crate::mqtt_log` and `::some_crate::sink` both work.
//!
//! Because the extra sink formats with `core::fmt`, every argument of a log
//! statement has to implement `core::fmt` as well as the selected back end's
//! trait. A type that only implements `defmt::Format` cannot be rendered by
//! the sink.
//!
//! # Why a proc macro
//!
//! `defmt` takes the file and line of a record from the DWARF entry of the
//! format-string static it interns (`defmt-macros`' `construct::interned_string`).
//! A `macro_rules!` wrapper would intern that static in the wrapper's file and
//! stamp every record with it. The tokens a proc macro emits carry the span of
//! its invocation, so the record keeps the location of the statement that
//! logged it.

use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use quote::quote;

/// Environment variable that names the extra sink's path.
const EXTRA_PREFIX: &str = "LOG_EXTRA_PREFIX";

/// Expands one level's macro.
fn expand(level: &str, input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    let level = format_ident!("{}", level);
    let consume = consume_unused(&input);
    let extra = extra_sink(&level, &input);

    quote! {{
        #[cfg(feature = "defmt")]
        ::defmt::#level!(#input);

        #[cfg(feature = "log")]
        ::log::#level!(#input);

        #consume

        #extra
    }}
    .into()
}

/// Emits the no-back-end arm, or nothing when `consume-unused` is off.
fn consume_unused(input: &TokenStream2) -> TokenStream2 {
    if !cfg!(feature = "consume-unused") {
        return TokenStream2::new();
    }

    quote! {
        // The calling crate has neither back end: use the arguments without
        // formatting them, so a value that only a log statement names is not
        // reported as unused.
        #[cfg(not(any(feature = "defmt", feature = "log")))]
        let _ = ::core::format_args!(#input);
    }
}

/// Emits the call into the extra sink, or nothing when it is not configured.
fn extra_sink(level: &Ident, input: &TokenStream2) -> TokenStream2 {
    let prefix = std::env::var(EXTRA_PREFIX).unwrap_or_default();
    let prefix = prefix.trim();

    if prefix.is_empty() {
        return TokenStream2::new();
    }

    let Ok(path) = prefix.parse::<TokenStream2>() else {
        let message = format!("`{EXTRA_PREFIX}` is not a valid path: `{prefix}`");
        return quote! { ::core::compile_error!(#message); };
    };

    // The feature is declared by the calling crate, so a build that does not
    // opt in removes this before the path is resolved.
    quote! {
        #[cfg(feature = "log-extra")]
        {
            #path::#level!(#input);
        }
    }
}

/// Logs at the trace level.
#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    expand("trace", input)
}

/// Logs at the debug level.
#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
    expand("debug", input)
}

/// Logs at the info level.
#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    expand("info", input)
}

/// Logs at the warn level.
#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
    expand("warn", input)
}

/// Logs at the error level.
#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    expand("error", input)
}
