/*!
This crate provides a library for extracting the publication date from
an article or a blog plost. It was heavily influenced by both the original
[article-date-extractor](https://github.com/Webhose/article-date-extractor)
written in Python, as well as its [Haskell port](https://github.com/amir/article-date-extractor).

# Usage

This crate is [on crates.io](https://crates.io/crates/article-date-extractor) and can be used by
adding `article-date-extractor` to your dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
article-date-extractor = "0.1"
```

and this to your crate root:

```rust
extern crate article_date_extractor;
```

# Example: extracting a date from a news article

```rust
use article_date_extractor::extract_date::extract_article_published_date;

let link = "http://edition.cnn.com/2015/11/28/opinions/sutter-cop21-paris-preview-two-degrees/index.html";
assert!(extract_article_published_date(&link, None).is_ok());
```

*/

#![recursion_limit = "1024"]
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate reqwest;
extern crate select;
extern crate rustc_serialize;
#[macro_use]
extern crate error_chain;
pub mod extract_date;
mod errors;
