//! Derive canonical SSZ codecs from Rust named structs and tagged enums.
mod attributes;
mod container;
#[cfg(test)]
mod tests;
mod union;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(Ssz, attributes(ssz))]
pub fn derive_ssz(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand(input: &DeriveInput) -> syn::Result<Tokens> {
    let path = crate_path(input)?;
    expand_for_crate(input, &path)
}

fn expand_for_crate(input: &DeriveInput, path: &Tokens) -> syn::Result<Tokens> {
    let attrs = attributes::parse(&input.attrs)?;
    if attrs.position.is_some() || attrs.selector.is_some() || attrs.rename.is_some() {
        return Err(Error::new_spanned(input, "invalid SSZ type attribute"));
    }
    let implementation = match &input.data {
        syn::Data::Struct(data) => container::expand(input, data, attrs.progressive, path)?,
        syn::Data::Enum(data) if !attrs.progressive => union::expand(input, data, path)?,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Ssz supports named structs and single-payload enums",
            ))
        }
    };
    Ok(quote!(const _: () = { #implementation };))
}

fn crate_path(input: &DeriveInput) -> syn::Result<Tokens> {
    let found = proc_macro_crate::crate_name("nyx-ssz").map_err(|_| {
        Error::new_spanned(input, "Ssz derive requires a direct nyx-ssz dependency")
    })?;
    Ok(match found {
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        proc_macro_crate::FoundCrate::Itself => quote!(::nyx_ssz),
    })
}

pub(crate) fn bounded_generics(
    input: &DeriveInput,
    types: &[&syn::Type],
    krate: &Tokens,
) -> syn::Generics {
    let mut generics = input.generics.clone();
    for ty in types {
        generics
            .make_where_clause()
            .predicates
            .push(syn::parse_quote!(#ty: #krate::codec::Ssz));
    }
    generics
}
