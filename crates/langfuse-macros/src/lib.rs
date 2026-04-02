//! Proc macros for the Langfuse SDK.
//!
//! Provides the [`observe`] attribute macro for zero-boilerplate tracing of
//! async and sync functions. Supports automatic span creation, input/output
//! capture, and stream/iterator wrapping.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Ident, ItemFn, Lit, Meta, MetaNameValue, Token, parse_macro_input};

/// Parsed arguments for the `#[observe(...)]` attribute.
struct ObserveArgs {
    name: Option<String>,
    as_type: Option<String>,
    capture_input: bool,
    capture_output: bool,
    transform_to_string: Option<String>,
}

impl Default for ObserveArgs {
    fn default() -> Self {
        Self {
            name: None,
            as_type: None,
            capture_input: true,
            capture_output: true,
            transform_to_string: None,
        }
    }
}

impl Parse for ObserveArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = ObserveArgs::default();

        if input.is_empty() {
            return Ok(args);
        }

        let pairs = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in pairs {
            match meta {
                Meta::NameValue(MetaNameValue {
                    path,
                    value:
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }),
                    ..
                }) => {
                    let ident = path
                        .get_ident()
                        .ok_or_else(|| syn::Error::new_spanned(&path, "expected identifier"))?;
                    let key = ident.to_string();
                    match key.as_str() {
                        "name" => args.name = Some(lit_str.value()),
                        "as_type" => {
                            let val = lit_str.value();
                            let valid_types = [
                                "span",
                                "generation",
                                "event",
                                "embedding",
                                "agent",
                                "tool",
                                "chain",
                                "retriever",
                                "evaluator",
                                "guardrail",
                            ];
                            if !valid_types.contains(&val.as_str()) {
                                return Err(syn::Error::new_spanned(
                                    &lit_str,
                                    format!("as_type must be one of: {}", valid_types.join(", "),),
                                ));
                            }
                            args.as_type = Some(val);
                        }
                        "transform_to_string" => {
                            args.transform_to_string = Some(lit_str.value());
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                ident,
                                format!("unknown observe attribute: `{key}`"),
                            ));
                        }
                    }
                }
                Meta::NameValue(MetaNameValue {
                    path,
                    value:
                        Expr::Lit(ExprLit {
                            lit: Lit::Bool(lit_bool),
                            ..
                        }),
                    ..
                }) => {
                    let ident = path
                        .get_ident()
                        .ok_or_else(|| syn::Error::new_spanned(&path, "expected identifier"))?;
                    let key = ident.to_string();
                    match key.as_str() {
                        "capture_input" => args.capture_input = lit_bool.value(),
                        "capture_output" => args.capture_output = lit_bool.value(),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                ident,
                                format!("unknown observe attribute: `{key}`"),
                            ));
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "expected `key = value` pair (e.g. `name = \"my-span\"`)",
                    ));
                }
            }
        }

        Ok(args)
    }
}

/// Detect whether the return type contains "Stream" or "Iterator".
///
/// Returns `Some("stream")`, `Some("iterator")`, or `None`.
fn detect_return_wrapper(sig: &syn::Signature) -> Option<&'static str> {
    let ret_type = match &sig.output {
        syn::ReturnType::Type(_, ty) => ty,
        syn::ReturnType::Default => return None,
    };

    let type_str = quote!(#ret_type).to_string();

    if type_str.contains("Stream") {
        Some("stream")
    } else if type_str.contains("Iterator") {
        Some("iterator")
    } else {
        None
    }
}

/// Instrument a function with Langfuse tracing.
///
/// # Attributes
///
/// - `#[observe]` — default: span name = function name, type = span
/// - `#[observe(name = "custom")]` — override span name
/// - `#[observe(as_type = "generation")]` — use generation instead of span
/// - `#[observe(capture_input = false)]` — skip serializing input args
/// - `#[observe(capture_output = false)]` — skip serializing output
/// - `#[observe(transform_to_string = "my_fn")]` — custom transform for stream/iterator output
///
/// # Stream / Iterator Support
///
/// If the return type contains `Stream`, the result is automatically wrapped in
/// `ObservingStream` which collects items and
/// finalizes the span when the stream completes.
///
/// Similarly, if the return type contains `Iterator`, the result is wrapped in
/// `ObservingIterator`.
///
/// # Example
///
/// ```ignore
/// use langfuse::observe;
///
/// #[observe]
/// async fn my_func(query: &str) -> String {
///     format!("result for {}", query)
/// }
/// ```
#[proc_macro_attribute]
pub fn observe(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ObserveArgs);
    let input_fn = parse_macro_input!(item as ItemFn);

    match expand_observe(args, input_fn) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_observe(
    args: ObserveArgs,
    mut input_fn: ItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    let span_name = args.name.unwrap_or_else(|| input_fn.sig.ident.to_string());
    let as_type = args.as_type.as_deref().unwrap_or("span");
    let is_async = input_fn.sig.asyncness.is_some();
    let return_wrapper = detect_return_wrapper(&input_fn.sig);

    // Collect parameter names for input capture (skip self params).
    let param_names: Vec<Ident> = input_fn
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg
                && let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref()
            {
                return Some(pat_ident.ident.clone());
            }
            None
        })
        .collect();

    // Build input capture code.
    let set_input = if args.capture_input && !param_names.is_empty() {
        let keys: Vec<String> = param_names.iter().map(|id| id.to_string()).collect();
        quote! {
            __langfuse_span.set_input(&::serde_json::json!({
                #( #keys: #param_names ),*
            }));
        }
    } else {
        quote! {}
    };

    // Build output capture code — only used for non-stream/iterator returns.
    let set_output = if args.capture_output && return_wrapper.is_none() {
        quote! {
            __langfuse_span.set_output(&__langfuse_result);
        }
    } else {
        quote! {}
    };

    let original_body = &input_fn.block;

    // Build the wrapping code for stream/iterator returns.
    let wrap_result = build_wrapper_code(return_wrapper, &args.transform_to_string);

    if is_async {
        // Async expansion: use the appropriate closure helper.
        let new_body = build_async_body(
            as_type,
            &span_name,
            &set_input,
            original_body,
            &set_output,
            &wrap_result,
            return_wrapper.is_some(),
        )?;
        input_fn.block = syn::parse2(new_body)?;
    } else {
        // Sync expansion: create span directly.
        let new_body = build_sync_body(
            as_type,
            &span_name,
            &set_input,
            original_body,
            &set_output,
            &wrap_result,
            return_wrapper.is_some(),
        )?;
        input_fn.block = syn::parse2(new_body)?;
    }

    Ok(quote! { #input_fn })
}

/// Build the wrapping code for stream/iterator returns.
///
/// When a stream or iterator is detected, instead of ending the span directly,
/// we wrap the result so the span is ended when the stream/iterator completes.
fn build_wrapper_code(
    return_wrapper: Option<&str>,
    transform_to_string: &Option<String>,
) -> proc_macro2::TokenStream {
    match return_wrapper {
        Some("stream") => {
            if let Some(transform_fn) = transform_to_string {
                let transform_ident: syn::Path =
                    syn::parse_str(transform_fn).expect("transform_to_string must be a valid path");
                quote! {
                    let __langfuse_result = ::langfuse::ObservingStream::with_transform(
                        __langfuse_span,
                        __langfuse_result,
                        #transform_ident,
                    );
                }
            } else {
                quote! {
                    let __langfuse_result = ::langfuse::ObservingStream::new(
                        __langfuse_span,
                        __langfuse_result,
                    );
                }
            }
        }
        Some("iterator") => {
            if let Some(transform_fn) = transform_to_string {
                let transform_ident: syn::Path =
                    syn::parse_str(transform_fn).expect("transform_to_string must be a valid path");
                quote! {
                    let __langfuse_result = ::langfuse::ObservingIterator::with_transform(
                        __langfuse_span,
                        __langfuse_result,
                        #transform_ident,
                    );
                }
            } else {
                quote! {
                    let __langfuse_result = ::langfuse::ObservingIterator::new(
                        __langfuse_span,
                        __langfuse_result,
                    );
                }
            }
        }
        _ => quote! {},
    }
}

/// Build the async expansion body for the given observation type.
fn build_async_body(
    as_type: &str,
    span_name: &str,
    set_input: &proc_macro2::TokenStream,
    original_body: &syn::Block,
    set_output: &proc_macro2::TokenStream,
    wrap_result: &proc_macro2::TokenStream,
    is_wrapper_return: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    // For stream/iterator returns, we don't end the span in the closure —
    // the wrapper handles it. We also need to move the span into the wrapper.
    let end_span = if is_wrapper_return {
        quote! {}
    } else {
        quote! { __langfuse_span.end(); }
    };

    // Map type string to the appropriate closure helper path.
    let helper_path = match as_type {
        "generation" => quote! { ::langfuse::langfuse_tracing::observe::with_generation },
        "agent" => quote! { ::langfuse::langfuse_tracing::observe::with_agent },
        "tool" => quote! { ::langfuse::langfuse_tracing::observe::with_tool },
        "chain" => quote! { ::langfuse::langfuse_tracing::observe::with_chain },
        "retriever" => quote! { ::langfuse::langfuse_tracing::observe::with_retriever },
        "evaluator" => quote! { ::langfuse::langfuse_tracing::observe::with_evaluator },
        "guardrail" => quote! { ::langfuse::langfuse_tracing::observe::with_guardrail },
        "embedding" => quote! { ::langfuse::langfuse_tracing::observe::with_embedding },
        // "span", "event", and any other type use with_observation with an explicit type.
        _ => {
            let obs_type = obs_type_token(as_type);
            return Ok(quote! {
                {
                    ::langfuse::langfuse_tracing::observe::with_observation(
                        #span_name,
                        #obs_type,
                        |__langfuse_span| async move {
                            #set_input
                            let __langfuse_result = #original_body;
                            #set_output
                            #wrap_result
                            #end_span
                            __langfuse_result
                        },
                    )
                    .await
                }
            });
        }
    };

    Ok(quote! {
        {
            #helper_path(
                #span_name,
                |__langfuse_span| async move {
                    #set_input
                    let __langfuse_result = #original_body;
                    #set_output
                    #wrap_result
                    #end_span
                    __langfuse_result
                },
            )
            .await
        }
    })
}

/// Build the sync expansion body for the given observation type.
fn build_sync_body(
    as_type: &str,
    span_name: &str,
    set_input: &proc_macro2::TokenStream,
    original_body: &syn::Block,
    set_output: &proc_macro2::TokenStream,
    wrap_result: &proc_macro2::TokenStream,
    is_wrapper_return: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let end_span = if is_wrapper_return {
        quote! {}
    } else {
        quote! { __langfuse_span.end(); }
    };

    let start_expr = match as_type {
        "generation" => quote! {
            ::langfuse::langfuse_tracing::generation::LangfuseGeneration::start(#span_name)
        },
        "embedding" => quote! {
            ::langfuse::langfuse_tracing::embedding::LangfuseEmbedding::start(#span_name)
        },
        _ => {
            let obs_type = obs_type_token(as_type);
            quote! {
                ::langfuse::langfuse_tracing::span::LangfuseSpan::start_with_type(#span_name, #obs_type)
            }
        }
    };

    Ok(quote! {
        {
            let __langfuse_span = #start_expr;
            #set_input
            let __langfuse_result = #original_body;
            #set_output
            #wrap_result
            #end_span
            __langfuse_result
        }
    })
}

/// Convert an as_type string to the corresponding `ObservationType` token.
fn obs_type_token(as_type: &str) -> proc_macro2::TokenStream {
    match as_type {
        "span" => quote! { ::langfuse_core::types::ObservationType::Span },
        "generation" => quote! { ::langfuse_core::types::ObservationType::Generation },
        "event" => quote! { ::langfuse_core::types::ObservationType::Event },
        "embedding" => quote! { ::langfuse_core::types::ObservationType::Embedding },
        "agent" => quote! { ::langfuse_core::types::ObservationType::Agent },
        "tool" => quote! { ::langfuse_core::types::ObservationType::Tool },
        "chain" => quote! { ::langfuse_core::types::ObservationType::Chain },
        "retriever" => quote! { ::langfuse_core::types::ObservationType::Retriever },
        "evaluator" => quote! { ::langfuse_core::types::ObservationType::Evaluator },
        "guardrail" => quote! { ::langfuse_core::types::ObservationType::Guardrail },
        // Unreachable due to validation in Parse impl, but default to Span.
        _ => quote! { ::langfuse_core::types::ObservationType::Span },
    }
}
