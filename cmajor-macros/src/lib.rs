use {
    cmajor_core::{Cmajor, ParseError},
    proc_macro2::{Span, TokenStream},
    quote::quote,
};

#[proc_macro]
pub fn cmajor(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let cmajor = Cmajor::new_from_env().unwrap();

    let tokens = TokenStream::from(tokens);
    let program = cmajor.parse(tokens.to_string());

    if let Err(ParseError::ParserError(err)) = program {
        return syn::Error::new(Span::call_site(), err.message())
            .into_compile_error()
            .into();
    }

    let tokens_string = tokens.to_string();
    quote! {
        {
            let cmajor = cmajor::Cmajor::new_from_env().unwrap();
            cmajor.parse(#tokens_string).expect("the cmajor program should be valid")
        }
    }
    .into()
}
