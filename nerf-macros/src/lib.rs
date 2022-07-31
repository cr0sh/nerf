extern crate proc_macro;

mod http;
mod rate_limited;
mod request;
mod tag;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Token, Visibility,
};

/// Extract name from `struct` or `enum`s, skipping attributes.
struct NamedItem {
    ident: Ident,
}

impl Parse for NamedItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _attrs = input.call(Attribute::parse_outer)?;
        let _: Visibility = input.parse()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![enum]) {
            input.parse::<Token![enum]>()?;
            let ret = Ok(NamedItem {
                ident: input.parse()?,
            });
            input.parse::<proc_macro2::TokenStream>()?;
            ret
        } else if lookahead.peek(Token![struct]) {
            input.parse::<Token![struct]>()?;
            let ret = Ok(NamedItem {
                ident: input.parse()?,
            });
            input.parse::<proc_macro2::TokenStream>()?;
            ret
        } else {
            Err(syn::Error::new(input.span(), "Expected `struct` or `enum`"))
        }
    }
}

pub(crate) trait PunctuatedExt {
    type Item;

    fn find_at_most_once<Target>(
        &self,
        predicate: fn(&Self::Item) -> Option<&Target>,
    ) -> Result<Option<&Target>, syn::Error>;
}

impl<Item, Token> PunctuatedExt for Punctuated<Item, Token>
where
    Item: Spanned,
{
    type Item = Item;

    fn find_at_most_once<Target>(
        &self,
        predicate: fn(&Item) -> Option<&Target>,
    ) -> Result<Option<&Target>, syn::Error> {
        let mut found = None;
        for x in self {
            if let Some(thing) = predicate(x) {
                if found.replace(thing).is_some() {
                    return Err(syn::Error::new(x.span(), "Duplicated attribute item"));
                }
            }
        }

        Ok(found)
    }
}

/// Attribute macro to set 'constant' weight for its rate limit.
///
/// For complex conditions, please manually implement [`nerf::WeightedRateLimit`].
///
/// # Example
///
/// ```
/// # use nerf_macros::rate_limited;
/// #[rate_limited(weight = 10)]
/// struct MyRequest {
///     params: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn rate_limited(attr: TokenStream, item: TokenStream) -> TokenStream {
    rate_limited::rate_limited(attr, item)
}

/// Attribute macro to implement `Request`.
///
/// # Example
///
/// ```
/// # use nerf_macros::request;
/// #[request(response = MyResponse)]
/// struct MyRequest {
///     params: String,
/// }
///
/// struct MyResponse;
/// ```
#[proc_macro_attribute]
pub fn request(attr: TokenStream, item: TokenStream) -> TokenStream {
    request::request(attr, item)
}

/// Attribute macro to implement `Request` and `HttpRequest` with GET method. Requests parameters are encoded
/// with [`serde_urlencoded`](https://docs.rs/serde_urlencoded).
///
/// - Endpoint is required with string literal.
/// - Setting `shim = false` will skip `impl TryFrom` for `Request` newtype.
///
/// # Example
///
/// ```
/// # use nerf_macros::get;
/// # trait Sealed {}
/// #[get("https://ifconfig.me", response = IfconfigResponse)]
/// struct Ifconfig;
/// struct IfconfigResponse;
/// ```
#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    http::entrypoint(attr, item, quote! { ::nerf::http::Method::GET })
}

/// Attribute macro to implement (`Request` or `JsonRequest`) and `HttpRequest` with POST method.
///
/// - Endpoint is required with string literal.
/// - Setting `shim = false` will skip `impl TryFrom` for `Request` newtype.
///
/// # Example
///
/// ```
/// # use nerf_macros::post;
/// # trait Sealed {}
/// #[post("https://ifconfig.me", response = IfconfigResponse)]
/// struct Ifconfig;
/// struct IfconfigResponse;
/// ```
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    http::entrypoint(attr, item, quote! { ::nerf::http::Method::POST })
}

/// Attribute macro to implement (`Request` or `JsonRequest`) and `HttpRequest` with PUT method.
///
/// - Endpoint is required with string literal.
/// - Setting `shim = false` will skip `impl TryFrom` for `Request` newtype.
///
/// # Example
///
/// ```
/// # use nerf_macros::put;
/// # trait Sealed {}
/// #[put("https://ifconfig.me", response = IfconfigResponse)]
/// struct Ifconfig;
/// struct IfconfigResponse;
/// ```
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    http::entrypoint(attr, item, quote! { ::nerf::http::Method::PUT })
}

/// Attribute macro to implement (`Request` or `JsonRequest`) and `HttpRequest` with DELETE method.
///
/// - Endpoint is required with string literal.
/// - Setting `shim = false` will skip `impl TryFrom` for `Request` newtype.
///
/// # Example
///
/// ```
/// # use nerf_macros::delete;
/// # trait Sealed {}
/// #[delete("https://ifconfig.me", response = IfconfigResponse)]
/// struct Ifconfig;
/// struct IfconfigResponse;
/// ```
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    http::entrypoint(attr, item, quote! { ::nerf::http::Method::GET })
}

/// Attribute macro to add a 'tag' to a type.
///
/// `#[tag(Foo = Bar)]` is transpiled into `impl Foo for TheType { type Foo = Bar; } `
#[proc_macro_attribute]
pub fn tag(attr: TokenStream, item: TokenStream) -> TokenStream {
    tag::tag(attr, item)
}
