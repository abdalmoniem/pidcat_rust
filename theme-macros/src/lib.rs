#![deny(clippy::unwrap_used)]

use proc_macro::TokenStream;
use proc_macro2::Literal;

use quote::quote;

use std::fs::read_dir;
use std::path::Path;

use syn::LitStr;
use syn::parse_macro_input;

#[proc_macro]
pub fn include_bundled_themes(input: TokenStream) -> TokenStream {
    let themes_dir = parse_macro_input!(input as LitStr);
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let themes_dir_path = Path::new(&manifest_dir).join(themes_dir.value());

    let mut theme_files = read_dir(&themes_dir_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", themes_dir_path.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|err| panic!("Failed to read theme entry: {err}"))
                .path()
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect::<Vec<_>>();

    theme_files.sort();

    let themes = theme_files.iter().map(|path| {
        let name = path
            .file_stem()
            .unwrap_or_else(|| panic!("Invalid theme file name: {path:?}"))
            .to_string_lossy()
            .into_owned();
        let name = LitStr::new(&name, proc_macro2::Span::call_site());
        let path = Literal::string(path.to_str().expect("Theme path is not valid UTF-8"));

        quote! {
            BundledTheme {
                name: #name,
                source: include_str!(#path),
            }
        }
    });

    quote! {
        /// generated at build time from `themes/*.toml`
        /// contains all the theme names and their paths
        pub const BUNDLED_THEMES: &[BundledTheme] = &[
            #(#themes),*
        ];
    }
    .into()
}
