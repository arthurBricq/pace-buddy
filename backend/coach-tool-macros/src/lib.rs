use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error, LitStr};

#[proc_macro_derive(CoachTool, attributes(tool))]
pub fn derive_coach_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let mut name = None::<LitStr>;
    let mut description = None::<LitStr>;

    for attr in input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("tool"))
    {
        let parse_result = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                name = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("description") {
                description = Some(meta.value()?.parse()?);
                return Ok(());
            }
            Err(meta.error("unsupported tool attribute; expected `name` or `description`"))
        });

        if let Err(err) = parse_result {
            return err.to_compile_error().into();
        }
    }

    let Some(name) = name else {
        return Error::new_spanned(&ident, "missing `#[tool(name = \"...\")]`")
            .to_compile_error()
            .into();
    };
    let Some(description) = description else {
        return Error::new_spanned(&ident, "missing `#[tool(description = \"...\")]`")
            .to_compile_error()
            .into();
    };

    quote! {
        impl crate::adapters::coach_tools::CoachToolArgs for #ident {
            const NAME: &'static str = #name;
            const DESCRIPTION: &'static str = #description;
        }
    }
    .into()
}
