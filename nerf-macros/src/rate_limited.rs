use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, LitInt, Token,
};

use crate::NamedItem;

struct RateLimitedAttr {
    weight: u64,
}

impl Parse for RateLimitedAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let weight_ident: Ident = input.parse()?;
        if weight_ident.to_string().as_str() != "weight" {
            return Err(syn::Error::new(
                weight_ident.span(),
                format!("Expected `weight`, got {weight_ident}"),
            ));
        }
        let _ = input.parse::<Token![=]>()?;
        let weight_value: LitInt = input.parse()?;
        Ok(Self {
            weight: weight_value.base10_parse()?,
        })
    }
}

pub fn rate_limited(attr: TokenStream, item: TokenStream) -> TokenStream {
    let RateLimitedAttr { weight } = parse_macro_input!(attr as RateLimitedAttr);
    let item_ = item.clone();
    let NamedItem { ident } = parse_macro_input!(item_ as NamedItem);
    let item = proc_macro2::TokenStream::from(item);
    quote! {
        #item

        impl ::nerf::WeightedRateLimit for #ident {
            fn weight(&self) -> u64 {
                #weight
            }
        }
    }
    .into()
}
