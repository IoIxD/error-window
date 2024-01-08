use proc_macro2::{TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{braced, Attribute, Signature, Visibility};

fn parse_knobs(input: ItemFn) -> TokenStream {
    let visiblity = input.vis.to_token_stream();
    let fn_token = input.sig.fn_token;
    let name = input.sig.ident.clone();
    let inputs_ = input.sig.inputs.to_token_stream();
    let ret_type = input.sig.output.to_token_stream();
    let inputs = inputs_;
    let body_ident = input.body().to_token_stream();
    let attrs = input
        .attrs()
        .map(|f| f.to_token_stream().to_string())
        .collect::<Vec<String>>()
        .join("\n")
        .to_token_stream();
    let body = quote! {
        #attrs
        #visiblity #fn_token #name(#inputs) #ret_type {
            let builder = std::thread::Builder::new().name("Main thread".to_string());
            let thread_handle = builder
                .spawn(move || {
                    let result = std::panic::catch_unwind(|| {
                        let m = || #ret_type {
                            #body_ident
                        };
                        if let Err(err) = m() {
                            error_window::dialog::Message::new(err.to_string())
                                .title("Error")
                                .show()
                                .expect("Could not display dialog box");
                        }
                    }); // will panic in fn2
                    if let Err(err) = result {
                        let mut er = format!("{:?}", err);

                        if let Some(s) = err.downcast_ref::<String>() {
                            er = format!("{}", s);
                        } else if let Some(s) = err.downcast_ref::<&str>() {
                            er = format!("{}", s);
                        } else {
                            er = format!("Unknown panic type \"{:?}\"", err.type_id())
                        }
                        error_window::dialog::Message::new(er)
                            .title("Error")
                            .show()
                            .expect("Could not display dialog box");
                    }
                })
                .expect("Asset loader thread spawn failed.");
            let stat = thread_handle.join();
            if let Err(e) = stat {
                error_window::dialog::Message::new(format!("Error: {:?}", e))
                    .title("Error")
                    .show()
                    .expect("Could not display dialog box");
            }
            Ok(())
        }
        use error_window::dialog::DialogBox;
    };

    println!("{}", body);
    body
}

fn token_stream_with_error(mut tokens: TokenStream, error: syn::Error) -> TokenStream {
    tokens.extend(error.into_compile_error());
    tokens
}

pub(crate) fn main(_args: TokenStream, item: TokenStream) -> TokenStream {
    // If any of the steps for this macro fail, we still want to expand to an item that is as close
    // to the expected output as possible. This helps out IDEs such that completions and other
    // related features keep working.
    let input: ItemFn = match syn::parse2(item.clone()) {
        Ok(it) => it,
        Err(e) => return token_stream_with_error(item, e),
    };
    parse_knobs(input)
}

struct ItemFn {
    outer_attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    brace_token: syn::token::Brace,
    inner_attrs: Vec<Attribute>,
    stmts: Vec<proc_macro2::TokenStream>,
}

impl ItemFn {
    /// Access all attributes of the function item.
    fn attrs(&self) -> impl Iterator<Item = &Attribute> {
        self.outer_attrs.iter().chain(self.inner_attrs.iter())
    }

    /// Get the body of the function item in a manner so that it can be
    /// conveniently used with the `quote!` macro.
    fn body(&self) -> Body<'_> {
        Body {
            brace_token: self.brace_token,
            stmts: &self.stmts,
        }
    }
}

impl Parse for ItemFn {
    #[inline]
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // This parse implementation has been largely lifted from `syn`, with
        // the exception of:
        // * We don't have access to the plumbing necessary to parse inner
        //   attributes in-place.
        // * We do our own statements parsing to avoid recursively parsing
        //   entire statements and only look for the parts we're interested in.

        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        let sig: Signature = input.parse()?;

        let content;
        let brace_token = braced!(content in input);
        let inner_attrs = Attribute::parse_inner(&content)?;

        let mut buf = proc_macro2::TokenStream::new();
        let mut stmts = Vec::new();

        while !content.is_empty() {
            if let Some(semi) = content.parse::<Option<syn::Token![;]>>()? {
                semi.to_tokens(&mut buf);
                stmts.push(buf);
                buf = proc_macro2::TokenStream::new();
                continue;
            }

            // Parse a single token tree and extend our current buffer with it.
            // This avoids parsing the entire content of the sub-tree.
            buf.extend([content.parse::<TokenTree>()?]);
        }

        if !buf.is_empty() {
            stmts.push(buf);
        }

        Ok(Self {
            outer_attrs,
            vis,
            sig,
            brace_token,
            inner_attrs,
            stmts,
        })
    }
}

struct Body<'a> {
    brace_token: syn::token::Brace,
    // Statements, with terminating `;`.
    stmts: &'a [TokenStream],
}

impl ToTokens for Body<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.brace_token.surround(tokens, |tokens| {
            for stmt in self.stmts {
                stmt.to_tokens(tokens);
            }
        });
    }
}
