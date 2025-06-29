#![warn(missing_docs)]

//! Procedural macro crate to accompany the linker-set crate.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::*;
use syn::punctuated::*;
use syn::*;

struct Name(String);

impl Parse for Name {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        assert!(name.len() == 1, "set_entry macro takes one argument");
        Ok(Self(name[0].to_string()))
    }
}

/// Attribute macro that puts an item into a linker set.
#[proc_macro_attribute]
pub fn set_entry(meta: TokenStream, decl: TokenStream) -> TokenStream {
    let meta = parse_macro_input!(meta as Name);
    let decl = parse_macro_input!(decl as ItemStatic);

    let set = meta.0;
    let set_section = format!("set_{set}");

    let g = quote! {
        #[unsafe(link_section = #set_section)]
        #[used]
        #decl
    };
    TokenStream::from(g)
}
