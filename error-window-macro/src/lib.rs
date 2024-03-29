use proc_macro::TokenStream;

mod entry;
#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    entry::main(args.into(), item.into()).into()
}
