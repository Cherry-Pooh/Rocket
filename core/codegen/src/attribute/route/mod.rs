mod parse;

use proc_macro2::{TokenStream, Span};
use devise::{Spanned, SpanWrapped, Result, FromMeta, Diagnostic};

use crate::{proc_macro2, syn};
use crate::proc_macro_ext::StringLit;
use crate::syn_ext::IdentExt;
use crate::http_codegen::{Method, Optional};

use crate::attribute::param::Guard;
use parse::{Route, Attribute, MethodAttribute};

impl Route {
    pub fn param_guards(&self) -> impl Iterator<Item = &Guard> {
        self.path_params.iter().filter_map(|p| p.guard())
    }

    pub fn query_guards(&self) -> impl Iterator<Item = &Guard> {
        self.query_params.iter().filter_map(|p| p.guard())
    }
}

fn query_decls(route: &Route) -> Option<TokenStream> {
    use devise::ext::{Split2, Split6};

    define_spanned_export!(Span::call_site() =>
        __req, __data, _log, _form, Outcome, _Ok, _Err, _Some, _None
    );

    // Record all of the static parameters for later filtering.
    let (raw_name, raw_value) = route.query_params.iter()
        .filter_map(|s| s.r#static())
        .map(|name| match name.find('=') {
            Some(i) => (&name[..i], &name[i + 1..]),
            None => (name.as_str(), "")
        })
        .split2();

    // Now record all of the dynamic parameters.
    let (name, matcher, ident, init_expr, push_expr, finalize_expr) = route.query_guards()
        .map(|guard| {
            let (name, ty) = (&guard.name, &guard.ty);
            let ident = guard.fn_ident.rocketized().with_span(ty.span());
            let matcher = match guard.trailing {
                true => quote_spanned!(name.span() => _),
                _ => quote!(#name)
            };

            define_spanned_export!(ty.span() => FromForm, _form);

            let ty = quote_spanned!(ty.span() => <#ty as #FromForm>);
            let init = quote_spanned!(ty.span() => #ty::init(#_form::Options::Lenient));
            let finalize = quote_spanned!(ty.span() => #ty::finalize(#ident));
            let push = match guard.trailing {
                true => quote_spanned!(ty.span() => #ty::push_value(&mut #ident, _f)),
                _ => quote_spanned!(ty.span() => #ty::push_value(&mut #ident, _f.shift())),
            };

            (name, matcher, ident, init, push, finalize)
        })
        .split6();

    #[allow(non_snake_case)]
    Some(quote! {
        let mut _e = #_form::Errors::new();
        #(let mut #ident = #init_expr;)*

        for _f in #__req.query_fields() {
            let _raw = (_f.name.source().as_str(), _f.value);
            let _key = _f.name.key_lossy().as_str();
            match (_raw, _key) {
                // Skip static parameters so <param..> doesn't see them.
                #(((#raw_name, #raw_value), _) => { /* skip */ },)*
                #((_, #matcher) => #push_expr,)*
                _ => { /* in case we have no trailing, ignore all else */ },
            }
        }

        #(
            let #ident = match #finalize_expr {
                #_Ok(_v) => #_Some(_v),
                #_Err(_err) => {
                    _e.extend(_err.with_name(#_form::NameView::new(#name)));
                    #_None
                },
            };
        )*

        if !_e.is_empty() {
            #_log::warn_("query string failed to match declared route");
            for _err in _e { #_log::warn_(_err); }
            return #Outcome::Forward(#__data);
        }

        #(let #ident = #ident.unwrap();)*
    })
}

fn request_guard_decl(guard: &Guard) -> TokenStream {
    let (ident, ty) = (guard.fn_ident.rocketized(), &guard.ty);
    define_spanned_export!(ty.span() => __req, __data, _request, FromRequest, Outcome);
    quote_spanned! { ty.span() =>
        let #ident: #ty = match <#ty as #FromRequest>::from_request(#__req).await {
            #Outcome::Success(__v) => __v,
            #Outcome::Forward(_) => return #Outcome::Forward(#__data),
            #Outcome::Failure((__c, _)) => return #Outcome::Failure(__c),
        };
    }
}

fn param_guard_decl(guard: &Guard) -> TokenStream {
    let (i, name, ty) = (guard.index, &guard.name, &guard.ty);
    define_spanned_export!(ty.span() =>
        __req, __data, _log, _None, _Some, _Ok, _Err,
        Outcome, FromSegments, FromParam
    );

    // Returned when a dynamic parameter fails to parse.
    let parse_error = quote!({
        #_log::warn_(&format!("Failed to parse '{}': {:?}", #name, __error));
        #Outcome::Forward(#__data)
    });

    // All dynamic parameters should be found if this function is being called;
    // that's the point of statically checking the URI parameters.
    let expr = match guard.trailing {
        false => quote_spanned! { ty.span() =>
            match #__req.routed_segment(#i) {
                #_Some(__s) => match <#ty as #FromParam>::from_param(__s) {
                    #_Ok(__v) => __v,
                    #_Err(__error) => return #parse_error,
                },
                #_None => {
                    #_log::error("Internal invariant: dynamic parameter not found.");
                    #_log::error("Please report this error to the Rocket issue tracker.");
                    return #Outcome::Forward(#__data);
                }
            }
        },
        true => quote_spanned! { ty.span() =>
            match <#ty as #FromSegments>::from_segments(#__req.routed_segments(#i..)) {
                #_Ok(__v) => __v,
                #_Err(__error) => return #parse_error,
            }
        },
    };

    let ident = guard.fn_ident.rocketized();
    quote!(let #ident: #ty = #expr;)
}

fn data_guard_decl(guard: &Guard) -> TokenStream {
    let (ident, ty) = (guard.fn_ident.rocketized(), &guard.ty);
    define_spanned_export!(ty.span() => __req, __data, FromData, Outcome);

    quote_spanned! { ty.span() =>
        let #ident: #ty = match <#ty as #FromData>::from_data(#__req, #__data).await {
            #Outcome::Success(__d) => __d,
            #Outcome::Forward(__d) => return #Outcome::Forward(__d),
            #Outcome::Failure((__c, _)) => return #Outcome::Failure(__c),
        };
    }
}

fn internal_uri_macro_decl(route: &Route) -> TokenStream {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Keep a global counter (+ thread ID later) to generate unique ids.
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    // FIXME: Is this the right order? Does order matter?
    let uri_args = route.param_guards()
        .chain(route.query_guards())
        .map(|guard| (&guard.fn_ident, &guard.ty))
        .map(|(ident, ty)| quote!(#ident: #ty));

    // Generate entropy based on the route's metadata.
    let mut hasher = DefaultHasher::new();
    route.handler.sig.ident.hash(&mut hasher);
    route.attr.uri.path().hash(&mut hasher);
    route.attr.uri.query().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    COUNTER.fetch_add(1, Ordering::AcqRel).hash(&mut hasher);

    let macro_name = route.handler.sig.ident.prepend(crate::URI_MACRO_PREFIX);
    let inner_macro_name = macro_name.append(&hasher.finish().to_string());
    let route_uri = route.attr.uri.to_string();

    quote_spanned! { Span::call_site() =>
        #[doc(hidden)]
        #[macro_export]
        /// Rocket generated URI macro.
        macro_rules! #inner_macro_name {
            ($($token:tt)*) => {{
                extern crate std;
                extern crate rocket;
                rocket::rocket_internal_uri!(#route_uri, (#(#uri_args),*), $($token)*)
            }};
        }

        #[doc(hidden)]
        pub use #inner_macro_name as #macro_name;
    }
}

fn responder_outcome_expr(route: &Route) -> TokenStream {
    let ret_span = match route.handler.sig.output {
        syn::ReturnType::Default => route.handler.sig.ident.span(),
        syn::ReturnType::Type(_, ref ty) => ty.span().into()
    };

    let user_handler_fn_name = &route.handler.sig.ident;
    let parameter_names = route.arguments.map.values()
        .map(|(ident, _)| ident.rocketized());

    let _await = route.handler.sig.asyncness
        .map(|a| quote_spanned!(a.span().into() => .await));

    define_spanned_export!(ret_span => __req, _handler);
    quote_spanned! { ret_span =>
        let ___responder = #user_handler_fn_name(#(#parameter_names),*) #_await;
        #_handler::Outcome::from(#__req, ___responder)
    }
}

fn codegen_route(route: Route) -> Result<TokenStream> {
    use crate::exports::*;

    // Generate the declarations for all of the guards.
    let request_guards = route.request_guards.iter().map(request_guard_decl);
    let param_guards = route.param_guards().map(param_guard_decl);
    let query_guards = query_decls(&route);
    let data_guard = route.data_guard.as_ref().map(data_guard_decl);

    // Gather info about the function.
    let (vis, handler_fn) = (&route.handler.vis, &route.handler);
    let handler_fn_name = &handler_fn.sig.ident;
    let internal_uri_macro = internal_uri_macro_decl(&route);
    let responder_outcome = responder_outcome_expr(&route);

    let method = route.attr.method;
    let path = route.attr.uri.to_string();
    let rank = Optional(route.attr.rank);
    let format = Optional(route.attr.format.as_ref());

    Ok(quote! {
        #handler_fn

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        /// Rocket code generated proxy structure.
        #vis struct #handler_fn_name {  }

        /// Rocket code generated proxy static conversion implementation.
        impl From<#handler_fn_name> for #StaticRouteInfo {
            #[allow(non_snake_case, unreachable_patterns, unreachable_code)]
            fn from(_: #handler_fn_name) -> #StaticRouteInfo {
                fn monomorphized_function<'_b>(
                    #__req: &'_b #Request,
                    #__data: #Data
                ) -> #HandlerFuture<'_b> {
                    #_Box::pin(async move {
                        #(#request_guards)*
                        #(#param_guards)*
                        #query_guards
                        #data_guard

                        #responder_outcome
                    })
                }

                #StaticRouteInfo {
                    name: stringify!(#handler_fn_name),
                    method: #method,
                    path: #path,
                    handler: monomorphized_function,
                    format: #format,
                    rank: #rank,
                }
            }
        }

        /// Rocket code generated proxy conversion implementation.
        impl From<#handler_fn_name> for #Route {
            #[inline]
            fn from(_: #handler_fn_name) -> #Route {
                #StaticRouteInfo::from(#handler_fn_name {}).into()
            }
        }

        /// Rocket code generated wrapping URI macro.
        #internal_uri_macro
    }.into())
}

fn complete_route(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let function: syn::ItemFn = syn::parse2(input)
        .map_err(|e| Diagnostic::from(e))
        .map_err(|diag| diag.help("`#[route]` can only be used on functions"))?;

    let attr_tokens = quote!(route(#args));
    let attribute = Attribute::from_meta(&syn::parse2(attr_tokens)?)?;
    codegen_route(Route::from(attribute, function)?)
}

fn incomplete_route(
    method: crate::http::Method,
    args: TokenStream,
    input: TokenStream
) -> Result<TokenStream> {
    let method_str = method.to_string().to_lowercase();
    // FIXME(proc_macro): there should be a way to get this `Span`.
    let method_span = StringLit::new(format!("#[{}]", method), Span::call_site())
        .subspan(2..2 + method_str.len());

    let method_ident = syn::Ident::new(&method_str, method_span.into());

    let function: syn::ItemFn = syn::parse2(input)
        .map_err(|e| Diagnostic::from(e))
        .map_err(|d| d.help(format!("#[{}] can only be used on functions", method_str)))?;

    let full_attr = quote!(#method_ident(#args));
    let method_attribute = MethodAttribute::from_meta(&syn::parse2(full_attr)?)?;

    let attribute = Attribute {
        method: SpanWrapped {
            full_span: method_span, key_span: None, span: method_span, value: Method(method)
        },
        uri: method_attribute.uri,
        data: method_attribute.data,
        format: method_attribute.format,
        rank: method_attribute.rank,
    };

    codegen_route(Route::from(attribute, function)?)
}

pub fn route_attribute<M: Into<Option<crate::http::Method>>>(
    method: M,
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream
) -> TokenStream {
    let result = match method.into() {
        Some(method) => incomplete_route(method, args.into(), input.into()),
        None => complete_route(args.into(), input.into())
    };

    result.unwrap_or_else(|diag| diag.emit_as_item_tokens())
}
