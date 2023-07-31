use proc_macro::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned};

pub fn test(_ats: TokenStream, ts: TokenStream) -> TokenStream {
    let func = parse_macro_input!(ts as syn::ItemFn);
    let syn::ItemFn { attrs, sig, .. } = func.clone();

    let func_name = sig.ident;
    for attr in attrs {
        if attr.path().is_ident("should_panic") {
            attr.span()
                .unwrap()
                .error("the #[should_panic] attribute is not currently supported")
                .emit();
        }
    }

    let quiet = func_name.to_string().starts_with("bindgen");
    let case_name = format_ident!("__test_case_{}", func_name);
    let quiet = if quiet {
        syn::LitBool::new(true, Span::call_site().into())
    } else {
        syn::LitBool::new(false, Span::call_site().into())
    };
    quote! {
        #func

        #[test_case]
        static #case_name: crate::test::Case = crate::test::Case {
            name: stringify!(#func_name),
            func: #func_name,
            quiet: #quiet,
        };
    }
    .into()
}
