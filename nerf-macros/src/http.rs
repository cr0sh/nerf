use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    LitStr, Token, Type,
};

use crate::{NamedItem, PunctuatedExt};

struct HttpAttr {
    endpoint: LitStr,
    response: Type,
}

impl Parse for HttpAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.parse_terminated::<_, Token![,]>(HttpAttrKind::parse)?;
        // FIXME: `input.span()` may be incorrect?
        let endpoint = attrs
            .find_at_most_once(|x| {
                if let HttpAttrKind::Endpoint(x) = x {
                    Some(x)
                } else {
                    None
                }
            })?
            .ok_or_else(|| syn::Error::new(input.span(), "endpoint is required"))?
            .clone();
        let response = *attrs
            .find_at_most_once(|x| {
                if let HttpAttrKind::Response(x) = x {
                    Some(x)
                } else {
                    None
                }
            })?
            .ok_or_else(|| syn::Error::new(input.span(), "response is required"))?
            .clone();

        endpoint.value().parse::<http::uri::Uri>().map_err(|e| {
            syn::Error::new(
                endpoint.span(),
                format!("endpoint is not a valid HTTP URI: {e}"),
            )
        })?;

        Ok(HttpAttr { endpoint, response })
    }
}

enum HttpAttrKind {
    Endpoint(LitStr),
    Response(Box<Type>),
}

impl Parse for HttpAttrKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            return Ok(HttpAttrKind::Endpoint(input.parse()?));
        }

        let key: Ident = input.parse()?;
        match key.to_string().as_str() {
            "response" => {
                input
                    .parse::<Token![=]>()
                    .map_err(|e| syn::Error::new(e.span(), "expected `=`"))?;
                Ok(HttpAttrKind::Response(input.parse()?))
            }
            other => Err(syn::Error::new(
                key.span(),
                format!("unexpected key {other}"),
            )),
        }
    }
}

impl Spanned for HttpAttrKind {
    fn span(&self) -> proc_macro2::Span {
        match self {
            HttpAttrKind::Endpoint(x) => x.span(),
            HttpAttrKind::Response(x) => x.span(),
        }
    }
}

pub fn entrypoint(
    attr: TokenStream,
    item: TokenStream,
    method: proc_macro2::TokenStream,
) -> TokenStream {
    let HttpAttr { endpoint, response } = parse_macro_input!(attr as HttpAttr);
    let item_ = item.clone();
    let NamedItem { ident } = parse_macro_input!(item_ as NamedItem);
    let item = proc_macro2::TokenStream::from(item);

    let request_impl = quote! {
        impl ::nerf::Request for #ident {
            type Response = #response;
        }
    };

    let http_request_impl = quote! {
        impl ::nerf::HttpRequest for #ident {
            fn method(&self) -> ::nerf::http::Method {
                #method
            }
            fn uri(&self) -> ::nerf::http::Uri {
                #endpoint.parse().expect("proc-macro attribute `endpoint` is an invalid HTTP URI")
            }
        }
    };

    let request_shim_impl = quote! {
        impl<T> ::core::convert::TryFrom<#ident> for Request<T>
        where
            T: ::core::convert::TryFrom<Request<#ident>>,
        {
            type Error = <T as ::core::convert::TryFrom<Request<#ident>>>::Error;

            fn try_from(
                value: #ident,
            ) -> Result<Self, <Self as ::core::convert::TryFrom<#ident>>::Error> {
                ::core::convert::TryFrom::try_from(Request(value)).map(Request)
            }
        }
    };

    quote! {
        #item

        #request_impl

        #http_request_impl

        #request_shim_impl
    }
    .into()
}
