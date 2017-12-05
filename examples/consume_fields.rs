#[macro_use]
extern crate darling;

#[macro_use]
extern crate quote;
extern crate syn;

use darling::ast;
use quote::{Tokens, ToTokens};

/// A speaking volume. Deriving `FromMetaItem` will cause this to be usable
/// as a string value for a meta-item key.
#[derive(Debug, Clone, Copy, FromMetaItem)]
#[darling(default)]
enum Volume {
    Normal,
    Whisper,
    Shout,
}

impl Default for Volume {
    fn default() -> Self {
        Volume::Normal
    }
}

/// Support parsing from a full derive input. Unlike FromMetaItem, this isn't
/// composable; each darling-dependent crate should have its own struct to handle
/// when its trait is derived.
#[derive(Debug, FromDeriveInput)]
// This line says that we want to process all attributes declared with `my_trait`,
// and that darling should panic if this receiver is given an enum.
#[darling(attributes(my_trait), supports(struct_any))]
struct MyInputReceiver {
    /// The struct ident.
    ident: syn::Ident,

    /// The type's generics. You'll need these any time your trait is expected
    /// to work with types that declare generics.
    generics: syn::Generics,

    /// Receives the body of the struct or enum. We don't care about
    /// struct fields because we previously told darling we only accept structs.
    body: ast::Body<(), MyFieldReceiver>,

    /// The Input Receiver demands a volume, so use `Volume::Normal` if the
    /// caller doesn't provide one.
    #[darling(default)]
    volume: Volume,
}

impl ToTokens for MyInputReceiver {
    fn to_tokens(&self, tokens: &mut Tokens) {
        let MyInputReceiver {
            ref ident,
            ref generics,
            ref body,
            volume,
        } = *self;

        let (imp, ty, wher) = generics.split_for_impl();
        let fields = body.as_ref()
            .take_struct()
            .expect("Should never be enum")
            .fields;

        // Generate the format string which shows each field and its name
        let fmt_string = fields
            .iter()
            .map(|f| format!("{:?} = {{}}", f.ident))
            .collect::<Vec<_>>()
            .join(", ");

        // Generate the actual values to fill the format string.
        let field_list = fields
            .into_iter()
            .map(|f| {
                let field_volume = f.volume.unwrap_or(volume);
                // TODO, handle tuple structs
                let ident = f.ident.as_ref().unwrap();
                match field_volume {
                    Volume::Normal => quote!(self.#ident),
                    Volume::Shout => {
                        quote!(::std::string::ToString::to_string(&self.#ident).to_uppercase())
                    }
                    Volume::Whisper => {
                        quote!(::std::string::ToString::to_string(&self.#ident).to_lowercase())
                    }
                }
            })
            .collect::<Vec<_>>();

        tokens.append(quote! {
            impl #imp Speak for #ident #ty #wher {
                fn speak(&self, writer: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    write!(#fmt_string, #(#field_list),*)
                }
            }
        });
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(my_trait))]
struct MyFieldReceiver {
    /// Get the ident of the field. For fields in tuple or newtype structs or
    /// enum bodies, this can be `None`.
    ident: Option<syn::Ident>,

    /// This magic field name pulls the type from the input.
    ty: syn::Ty,

    /// We declare this as an `Option` so that during tokenization we can write
    /// `field.volume.unwrap_or(derive_input.volume)` to facilitate field-level
    /// overrides of struct-level settings.
    volume: Option<Volume>,
}

fn main() {
    println!(r#"
View example source to see why this code would work:

#[derive(MyTrait)]
#[my_trait(volume = "shout")]
pub struct Foo {{
    #[my_trait(volume = "whisper")]
    bar: bool,

    baz: i64,
}}
    "#);
}