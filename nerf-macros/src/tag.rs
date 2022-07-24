use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse_macro_input, Ident, Token, Type};

use crate::NamedItem;

struct TagAttrParam {
    key: Ident,
    value: Type,
}

impl Parse for TagAttrParam {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        let _: Token![=] = input.parse()?;
        let value = input.parse()?;
        Ok(Self { key, value })
    }
}

struct TagParams {
    attrs: Vec<TagAttrParam>,
}

impl Parse for TagParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input
            .parse_terminated::<_, Token![,]>(TagAttrParam::parse)?
            .into_iter()
            .collect();

        Ok(Self { attrs })
    }
}

pub fn tag(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_ = item.clone();
    let item: proc_macro2::TokenStream = item.into();
    let NamedItem { ident } = parse_macro_input!(item_ as NamedItem);
    let impls = parse_macro_input!(attr as TagParams)
        .attrs
        .iter()
        .map(|x| {
            let TagAttrParam { key, value } = x;
            quote! {
                impl #key for #ident {
                    type #key = #value;
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #item

        #(#impls)*
    }
    .into()
}
