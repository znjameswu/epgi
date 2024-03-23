//! Define multiple default implementations for a trait.
//!
//! This library contains two attribute macros: `default_trait_impl` which defines a default trait
//! implementation, and `trait_impl` which uses a default trait implementation you've defined.
//!
//! This is particularly useful in testing, when many of your mocked types will have very similar
//! trait implementations, but do not want the canonical default trait implementation to use mocked
//! values.
//!
//! # Example
//!
//! First, define a default trait implementation for the trait `Car`:
//!
//! ```
//! #[default_trait_impl]
//! impl Car for NewCar {
//!     fn get_mileage(&self) -> Option<usize> { Some(6000) }
//!     fn has_bluetooth(&self) -> bool { true }
//! }
//! ```
//!
//! `NewCar` does not need to be defined beforehand.
//!
//! Next, implement the new default implementation for a type:
//!
//! ```
//! struct NewOldFashionedCar;
//!
//! #[trait_impl]
//! impl NewCar for NewOldFashionedCar {
//!     fn has_bluetooth(&self) -> bool { false }
//! }
//!
//!
//! struct WellUsedNewCar;
//!
//! #[trait_impl]
//! impl NewCar for WellUsedNewCar {
//!     fn get_mileage(&self) -> Option<usize> { Some(100000) }
//! }
//! ```
//!
//! This will ensure that our structs use the `NewCar` defaults, without having to change the
//! canonical `Car` default implementation:
//!
//! ```
//! fn main() {
//!     assert_eq!(NewOldFashionedCar.get_mileage(), Some(6000));
//!     assert_eq!(NewOldFashionedCar.has_bluetooth(), false);
//!     assert_eq!(WellUsedNewCar.get_mileage(), Some(100000));
//!     assert_eq!(WellUsedNewCar.has_bluetooth(), true);
//! }
//! ```

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::sync::Mutex;
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, parse_str, Ident, ImplItem, ItemImpl, ItemStruct, Path, Token, Type,
    TypePath,
};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref DERIVE_TEMPLATES: Mutex<HashMap<String, Vec<DeriveTemplate>>> =
        Mutex::new(HashMap::new());
}

struct DeriveTemplate {
    pub trait_name: String,
    pub items: Vec<String>,
}

#[proc_macro_attribute]
pub fn default_derive(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemImpl);

    let pseudotrait = match *input.self_ty {
        Type::Path(type_path) => match type_path.path.get_ident() {
            Some(ident) => ident.to_string(),
            None => return syntax_invalid_error(),
        },
        _ => return syntax_invalid_error(),
    };

    let trait_name = match input.trait_ {
        Some(trait_tuple) => match trait_tuple.1.get_ident() {
            Some(ident) => ident.to_string(),
            None => return syntax_invalid_error(),
        },
        _ => return syntax_invalid_error(),
    };

    let items: Vec<String> = input
        .items
        .iter()
        .map(|item| {
            return quote! {
                #item
            }
            .to_string();
        })
        .collect();

    DERIVE_TEMPLATES
        .lock()
        .unwrap()
        .entry(pseudotrait)
        .or_insert_with(|| Vec::new())
        .push(DeriveTemplate { trait_name, items });

    TokenStream::new()
}

fn syntax_invalid_error() -> TokenStream {
    return quote! {
        compile_error!("`default_trait_impl` expects to be given a syntactially valid trait implementation");
    }.into();
}

#[proc_macro_attribute]
pub fn derive_from(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated<Path, Token![,]>::parse_terminated);

    let struct_item = parse_macro_input!(input as ItemStruct);

    let mut trait_impls = Vec::new();

    let struct_ident = struct_item.ident.clone();

    for arg in args {
        let Some(trait_ident) = arg.get_ident() else {
            return syntax_invalid_error();
        };
        let trait_name = trait_ident.to_string();

        let guard = DERIVE_TEMPLATES.lock().unwrap();
        let Some(derive_templates) = guard.get(&trait_name) else {
            return syntax_invalid_error();
        };

        for derive_template in derive_templates.iter() {
            let trait_ident =
                Ident::new(&derive_template.trait_name, proc_macro2::Span::call_site());
            let trait_impl = ItemImpl {
                attrs: Vec::new(),
                defaultness: None,
                unsafety: None,
                impl_token: Default::default(),
                generics: Default::default(),
                trait_: Some((None, trait_ident.into(), Default::default())),
                self_ty: Box::new(Type::Path(TypePath {
                    qself: None,
                    path: struct_ident.clone().into(),
                })),
                brace_token: Default::default(),
                items: derive_template
                    .items
                    .iter()
                    .map(|item| {
                        let parsed_result: ImplItem = parse_str(item).unwrap();
                        parsed_result
                    })
                    .collect(),
            };
            trait_impls.push(trait_impl);
        }
    }

    quote! {
        #struct_item

        #(
            #trait_impls
        )*
    }
    .into()
}
