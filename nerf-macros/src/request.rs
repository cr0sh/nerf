use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Token,
};

use crate::NamedItem;

struct RequestAttr {
    resp_ty: Ident,
}

impl Parse for RequestAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let response_ident: Ident = input.parse()?;
        if response_ident.to_string().as_str() != "response" {
            return Err(syn::Error::new(
                response_ident.span(),
                format!("Expected `response`, got {response_ident}"),
            ));
        }
        let _ = input.parse::<Token![=]>()?;
        let resp_ty = input.parse()?;
        Ok(Self { resp_ty })
    }
}

pub fn request(attr: TokenStream, item: TokenStream) -> TokenStream {
    let RequestAttr { resp_ty } = parse_macro_input!(attr as RequestAttr);
    let item_ = item.clone();
    let NamedItem { ident } = parse_macro_input!(item_ as NamedItem);
    let item = proc_macro2::TokenStream::from(item);
    quote! {
        #item

        impl ::nerf::Request for #ident {
            type Response = #resp_ty;
        }
    }
    .into()
}
