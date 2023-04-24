use crate::code_preprocessor::CodePreprocessor;
use clap::{Arg, ArgMatches, Command};
use mdbook::book::Book;
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};
use mdbook::BookItem;
use regex::Regex;
use semver::{Version, VersionReq};
use std::io;
use std::process;

pub fn make_app() -> Command {
    Command::new("code-preprocessor")
        .about("A custom mdbook preprocessor for code highlighting")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

fn main() {
    let matches = make_app().get_matches();

    let preprocessor = CodePreprocessor::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

    if !version_req.matches(&book_version) {
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args
        .get_one::<String>("renderer")
        .expect("Required argument");
    let supported = pre.supports_renderer(renderer);

    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

mod code_preprocessor {
    use super::*;

    pub struct CodePreprocessor;

    impl CodePreprocessor {
        pub fn new() -> CodePreprocessor {
            CodePreprocessor
        }
    }

    impl Preprocessor for CodePreprocessor {
        fn name(&self) -> &str {
            "code-preprocessor"
        }

        fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
            let re = Regex::new(r"(?m)```((?:.|\n)*?)```").unwrap();

            book.for_each_mut(|section: &mut BookItem| {
                if let BookItem::Chapter(ref mut ch) = *section {
                    ch.content = re
                        .replace_all(&ch.content, |caps: &regex::Captures| {
                            if !caps[0].contains("%%") {
                                caps[0].to_string()
                            }
                            else {
                                let mut result = "<pre><code>".to_owned() + &caps[1].trim() + "\n</code></pre>\n";
                                while result.contains("%%") {
                                    result = result.replacen("%%", "<span class=\"code-user-input\">", 1);
                                    result = result.replacen("%%", "</span>", 1);
                                }
                                result
                            }
                        })
                        .to_string();
                }
            });

            Ok(book)
        }

        fn supports_renderer(&self, renderer: &str) -> bool {
            renderer == "html"
        }
    }
}
