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

#[proc_macro_attribute]
pub fn set_entry(meta: TokenStream, decl: TokenStream) -> TokenStream {
    let meta = parse_macro_input!(meta as Name);
    let decl = parse_macro_input!(decl as ItemStatic);

    let set = meta.0;
    let set_section = format!("set_{}", set);
    let set_ident = format_ident!("{}", set);
    let start_set = format_ident!("__start_set_{}", set);
    let fn_name = format_ident!(
        "__set_{}_typecheck_{}",
        set,
        decl.ident.to_string().to_lowercase()
    );
    let expr = decl.expr.clone();

    let gen = quote! {
        #[link_section = #set_section]
        #[used]
        #decl

        #[cfg(any(debug, test))]
        #[allow(unused)]
        fn #fn_name() -> bool {
            // for typechecking
            unsafe { #set_ident::#start_set == #expr }
        }
    };
    TokenStream::from(gen)
}
