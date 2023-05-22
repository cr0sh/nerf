use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use regex::Regex;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    LitBool, LitStr, Path, Token, Type,
};

use crate::{NamedItem, PunctuatedExt};

#[derive(Clone, Debug)]
enum Shim {
    Bool(LitBool),
    Path(Path),
}

impl Parse for Shim {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.lookahead1().peek(LitBool) {
            input.parse().map(Self::Bool)
        } else {
            input.parse().map(Self::Path)
        }
    }
}

impl Spanned for Shim {
    fn span(&self) -> proc_macro2::Span {
        match self {
            Shim::Bool(x) => x.span(),
            Shim::Path(x) => x.span(),
        }
    }
}

struct HttpAttr {
    endpoint: LitStr,
    response: Type,
    shim: Option<Shim>,
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
            shim,
        })
    }
}

enum HttpAttrKind {
    Endpoint(LitStr),
    Response(Type),
    Signer(Ident),
    Shim(Shim),
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
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\{[a-zA-Z_][a-zA-Z0-9_]*?\}"#).unwrap());
    let mut fields = Vec::new();
    while let Some(m) = RE.find(&raw) {
        let range = m.range();
        assert!(range.len() > 2);
        fields.push(raw[(range.start + 1)..(range.end - 1)].to_string());
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
        "http://foo/{bar}/{baz}",
        "http://foo/{}/{}",
        &["bar", "baz"],
    );
    case(
        "http://foo/{bar}/{baz}/qux",
        "http://foo/{}/{}/qux",
        &["bar", "baz"],
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
        shim: _shim,
    } = parse_macro_input!(attr as HttpAttr);
    let item_ = item.clone();
    let NamedItem { ident } = parse_macro_input!(item_ as NamedItem);
    let item = proc_macro2::TokenStream::from(item);

    // let shim = match shim {
    //     Some(Shim::Bool(bool)) => {
    //         if bool.value {
    //             return syn::Error::new(bool.span(), "attribute parameter `shim` cannot be `true`")
    //                 .into_compile_error()
    //                 .into();
    //         }

    //         None
    //     }
    //     Some(Shim::Path(path)) => Some(path.to_token_stream()),
    //     None => Some(quote! { __private }),
    // };

    if endpoint.value().contains("{}") {
        return syn::Error::new(endpoint.span(), "endpoint must not contain `{}`\nIf you meant a place for format arguments, use `{field_name}` instead")
            .into_compile_error()
            .into();
    }

    let (sub, args) = parse_endpoint(endpoint.value());
    let args = args
        .into_iter()
        .map(|arg| {
            let ident = Ident::new(&arg, endpoint.span());
            quote!(self.#ident)
        })
        .collect::<Vec<_>>();
    let sub = LitStr::new(&sub, endpoint.span());

    let endpoint = quote! {
        format!(#sub, #(#args),*)
    };

    quote! {
        #item

        impl ::nerf::Request for #ident {
            type Response = #response;
        }

        impl ::nerf::HttpRequest for #ident {
            fn method(&self) -> ::nerf::http::Method {
                #method
            }
            fn uri(&self) -> ::nerf::http::Uri {
                #endpoint.parse().expect("proc-macro attribute `endpoint` is an invalid HTTP URI")
            }
        }

        impl Sealed for #ident {}
    }
    .into()
}
