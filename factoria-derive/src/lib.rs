//! Derive macro for the `factoria` test factory library.
//!
//! Use it through `factoria` with the `derive` feature, not directly.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Expr, Fields, LitStr};

/// Generates a `<Name>Factory` struct implementing `factoria::Factory`.
///
/// Fields with `#[factory(default = "<expr>")]` use that expression (which
/// may refer to `seq`); every other field gets a random value from `fake`.
#[proc_macro_derive(Factory, attributes(factory))]
pub fn derive_factory(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;
    let factory_name = format_ident!("{}Factory", name);

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "#[derive(Factory)] does not support generic structs yet",
        ));
    }

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => return Err(syn::Error::new_spanned(name, "Factory needs named fields")),
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Factory can only be derived for structs",
            ))
        }
    };

    let mut inits = Vec::new();
    for field in fields {
        let ident = field.ident.as_ref().expect("named field");
        let mut custom: Option<Expr> = None;

        for attr in &field.attrs {
            if !attr.path().is_ident("factory") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("default") {
                    let lit: LitStr = meta.value()?.parse()?;
                    custom = Some(lit.parse()?);
                    Ok(())
                } else {
                    Err(meta.error("expected `default = \"<expr>\"`"))
                }
            })?;
        }

        let value = match custom {
            Some(expr) => quote!(#expr),
            None => quote!(::factoria::fake::Fake::fake(&::factoria::fake::Faker)),
        };
        inits.push(quote!(#ident: #value));
    }

    Ok(quote! {
        #vis struct #factory_name;

        impl ::factoria::Factory for #factory_name {
            type Model = #name;

            #[allow(unused_variables)]
            fn definition(seq: u64) -> Self::Model {
                #name { #(#inits),* }
            }
        }
    })
}
