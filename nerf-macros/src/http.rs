use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use regex::Regex;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    LitBool, LitStr, Token,
};

use crate::{NamedItem, PunctuatedExt};

struct HttpAttr {
    endpoint: LitStr,
    response: Ident,
    signer: Option<Ident>,
    shim: Option<LitBool>,
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
        let response = attrs
            .find_at_most_once(|x| {
                if let HttpAttrKind::Response(x) = x {
                    Some(x)
                } else {
                    None
                }
            })?
            .ok_or_else(|| syn::Error::new(input.span(), "response is required"))?
            .clone();
        let signer = attrs
            .find_at_most_once(|x| {
                if let HttpAttrKind::Signer(x) = x {
                    Some(x)
                } else {
                    None
                }
            })?
            .cloned();
        let shim = attrs
            .find_at_most_once(|x| {
                if let HttpAttrKind::Shim(x) = x {
                    Some(x)
                } else {
                    None
                }
            })?
            .cloned();

        endpoint.value().parse::<http::uri::Uri>().map_err(|e| {
            syn::Error::new(
                endpoint.span(),
                format!("endpoint is not a valid HTTP URI: {e}"),
            )
        })?;

        Ok(HttpAttr {
            endpoint,
            response,
            signer,
            shim,
        })
    }
}

enum HttpAttrKind {
    Endpoint(LitStr),
    Response(Ident),
    Signer(Ident),
    Shim(LitBool),
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
            "signer" => {
                input
                    .parse::<Token![=]>()
                    .map_err(|e| syn::Error::new(e.span(), "expected `=`"))?;
                Ok(HttpAttrKind::Signer(input.parse()?))
            }
            "shim" => {
                input
                    .parse::<Token![=]>()
                    .map_err(|e| syn::Error::new(e.span(), "expected `=`"))?;
                Ok(HttpAttrKind::Shim(input.parse()?))
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
            HttpAttrKind::Signer(x) => x.span(),
            HttpAttrKind::Shim(x) => x.span(),
        }
    }
}

/// Parses raw endpoint string into `format!`-able string and subsequent parameteres.
fn parse_endpoint(mut raw: String) -> (String, Vec<String>) {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#":[a-zA-Z_][a-zA-Z0-9_]*"#).unwrap());
    let mut fields = Vec::new();
    while let Some(m) = RE.find(&raw) {
        let range = m.range();
        assert!(range.len() > 1);
        fields.push(format!("self.{}", &raw[(range.start + 1)..range.end]));
        raw.replace_range(range, "{}");
    }
    (raw, fields)
}

#[test]
fn test_parse_endpoint() {
    fn case(s: &'static str, t: &'static str, u: &[&'static str]) {
        let (a, b) = parse_endpoint(String::from(s));
        assert_eq!(a, t);
        assert_eq!(b, u.iter().cloned().map(String::from).collect::<Vec<_>>());
    }

    case("foobarbaz", "foobarbaz", &[]);
    case("http://foo", "http://foo", &[]);
    case(
        "http://foo/:bar/:baz",
        "http://foo/{}/{}",
        &["self.bar", "self.baz"],
    );
    case(
        "http://foo/:bar/:baz/qux",
        "http://foo/{}/{}/qux",
        &["self.bar", "self.baz"],
    );
}

pub fn entrypoint(
    attr: TokenStream,
    item: TokenStream,
    method: proc_macro2::TokenStream,
) -> TokenStream {
    let HttpAttr {
        endpoint,
        response,
        signer,
        shim,
    } = parse_macro_input!(attr as HttpAttr);
    let item_ = item.clone();
    let NamedItem { ident } = parse_macro_input!(item_ as NamedItem);
    let item = proc_macro2::TokenStream::from(item);

    let request_impl = quote! {
        impl ::nerf::Request for #ident {
            type Response = #response;
        }
    };

    let signer = signer.map(|x| quote! { #x }).unwrap_or(quote! { () });
    let shim = shim.as_ref().map(LitBool::value).unwrap_or(true);

    if endpoint.value().contains("{}") {
        return syn::Error::new(endpoint.span(), "endpoint must not contain `{}`\nIf you meant a place for format arguments, use `:field_name` instead")
            .into_compile_error()
            .into();
    }

    let (sub, args) = parse_endpoint(endpoint.value());
    let sub = LitStr::new(&sub, endpoint.span());

    let endpoint = quote! {
        format!(#sub, #(#args),*)
    };

    let http_request_impl = quote! {
        impl ::nerf::HttpRequest for #ident {
            type Signer = #signer;
            fn method(&self) -> ::nerf::http::Method {
                #method
            }
            fn uri(&self) -> ::nerf::http::Uri {
                #endpoint.parse().expect("proc-macro attribute `endpoint` is an invalid HTTP URI")
            }
        }
    };

    let request_shim_impl = if shim {
        quote! {
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
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    quote! {
        #item

        #request_impl

        #http_request_impl

        #request_shim_impl
    }
    .into()
}
