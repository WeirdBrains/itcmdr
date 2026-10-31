//! `#[derive(ServicePluginError)]` (v0.15.0), re-exported as `itcmdr_common::ServicePluginError`.
//!
//! The struct must have named fields `error: String`, `criticality: ErrorCriticality`,
//! `backtrace: String` and `service_plugin: String`; a missing field or a wrong type is a compile
//! error. Every other field goes into the record's `context` (via `ToString`) and must implement
//! `Default` (for the generated constructor) and `Display`; mark one `#[service_plugin(skip)]` to
//! leave it out of the context.
//!
//! Container attributes: `#[service_plugin(name = "disk-free")]` fixes the plugin name (then
//! `new(error, criticality)`; without it `new(service_plugin, error, criticality)`), and
//! `#[service_plugin(kind = "...")]` overrides the error kind (default: the struct name).
//!
//! Generated: `Display` (the error message), `std::error::Error`,
//! `itcmdr_common::ServicePluginError`, and `new(..)`, which captures
//! `std::backtrace::Backtrace::force_capture()` into `backtrace`.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields, LitStr, Type};

const REQUIRED: [(&str, &str); 4] = [
    ("error", "String"),
    ("criticality", "ErrorCriticality"),
    ("backtrace", "String"),
    ("service_plugin", "String"),
];

#[proc_macro_derive(ServicePluginError, attributes(service_plugin))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn last_ident(t: &Type) -> Option<String> {
    match t {
        Type::Path(p) if p.qself.is_none() => p.path.segments.last().map(|s| {
            if s.arguments.is_empty() {
                s.ident.to_string()
            } else {
                String::new()
            }
        }),
        _ => None,
    }
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            "ServicePluginError cannot be derived for generic types",
        ));
    }
    let mut name: Option<LitStr> = None;
    let mut kind: Option<LitStr> = None;
    for a in &input.attrs {
        if !a.path().is_ident("service_plugin") {
            continue;
        }
        a.parse_nested_meta(|m| {
            if m.path.is_ident("name") {
                name = Some(m.value()?.parse()?);
                Ok(())
            } else if m.path.is_ident("kind") {
                kind = Some(m.value()?.parse()?);
                Ok(())
            } else {
                Err(m.error("expected `name = \"...\"` or `kind = \"...\"`"))
            }
        })?;
    }
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(n) => &n.named,
            _ => {
                return Err(Error::new_spanned(
                    ident,
                    "ServicePluginError needs a struct with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                ident,
                "ServicePluginError can only be derived for structs",
            ))
        }
    };
    for (f, ty) in REQUIRED {
        match fields
            .iter()
            .find(|x| x.ident.as_ref().is_some_and(|i| i == f))
        {
            None => {
                return Err(Error::new(
                    Span::call_site(),
                    format!("ServicePluginError needs a field `{f}: {ty}` on `{ident}`"),
                ))
            }
            Some(x) if last_ident(&x.ty).as_deref() != Some(ty) => {
                return Err(Error::new_spanned(
                    &x.ty,
                    format!("field `{f}` must be `{ty}`"),
                ))
            }
            Some(_) => {}
        }
    }
    let mut extras = Vec::new();
    let mut extra_inits = Vec::new();
    for f in fields {
        let fi = f.ident.as_ref().expect("named");
        if REQUIRED.iter().any(|(r, _)| fi == r) {
            continue;
        }
        let mut skip = false;
        for a in &f.attrs {
            if a.path().is_ident("service_plugin") {
                a.parse_nested_meta(|m| {
                    if m.path.is_ident("skip") {
                        skip = true;
                        Ok(())
                    } else {
                        Err(m.error("expected `skip`"))
                    }
                })?;
            }
        }
        extra_inits.push(quote! { #fi: ::core::default::Default::default() });
        if !skip {
            let key = fi.to_string();
            extras.push(quote! { m.insert(#key.to_string(), ::std::string::ToString::to_string(&self.#fi)); });
        }
    }
    let kind = kind.unwrap_or_else(|| LitStr::new(&ident.to_string(), ident.span()));
    let ctor = match &name {
        Some(n) => quote! {
            /// New error for this plugin; captures a backtrace of the caller.
            pub fn new(error: impl ::core::convert::Into<::std::string::String>, criticality: ::itcmdr_common::ErrorCriticality) -> Self {
                Self::with_plugin(#n, error, criticality)
            }
        },
        None => quote! {
            /// New error raised by `service_plugin`; captures a backtrace of the caller.
            pub fn new(service_plugin: impl ::core::convert::Into<::std::string::String>, error: impl ::core::convert::Into<::std::string::String>, criticality: ::itcmdr_common::ErrorCriticality) -> Self {
                Self::with_plugin(service_plugin, error, criticality)
            }
        },
    };
    Ok(quote! {
        impl #ident {
            #ctor
            fn with_plugin(service_plugin: impl ::core::convert::Into<::std::string::String>, error: impl ::core::convert::Into<::std::string::String>, criticality: ::itcmdr_common::ErrorCriticality) -> Self {
                Self {
                    error: error.into(),
                    criticality,
                    backtrace: ::itcmdr_common::capture_backtrace(),
                    service_plugin: service_plugin.into(),
                    #(#extra_inits,)*
                }
            }
        }
        impl ::core::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(&self.error)
            }
        }
        impl ::std::error::Error for #ident {}
        impl ::itcmdr_common::ServicePluginError for #ident {
            fn criticality(&self) -> ::itcmdr_common::ErrorCriticality { self.criticality }
            fn service_plugin(&self) -> &str { &self.service_plugin }
            fn backtrace_text(&self) -> &str { &self.backtrace }
            fn kind(&self) -> &str { #kind }
            fn context(&self) -> ::std::collections::BTreeMap<::std::string::String, ::std::string::String> {
                #[allow(unused_mut)]
                let mut m = ::std::collections::BTreeMap::new();
                #(#extras)*
                m
            }
        }
    })
}
