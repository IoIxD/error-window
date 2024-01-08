# error-window

Simple Rust proc macro that will make your main function output errors as dialog boxes as well as errors to the console. It will also check any panic/todo that Rust's [catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) will catch.

