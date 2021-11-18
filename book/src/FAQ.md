# FAQ

## Why is it called dyner?

Honestly? [dyno](https://crates.io/crates/dyno) was taken. Best name ever! Think of the logo! It practically draws itself!

But dyner is cool too: it's more dyn than dyn!

## How do I pronounce dyner, dude?

Pronounce it like "diner".

Or maybe "dinner".

Whatever floats your boat!

## Does `dyner` work for no-std crates?

Yes! But without access to the `Box` type, you can only pass trait objects that are stored in references.

## Wouldn't it be nicer if you built this stuff into the language?

Maybe! But first we have to decide what we want to build. There are also some aspects of trait objects that are necessarily quite "custom", so it may be that there would still be a role for a crate like `dyner` even if we were to change how `dyn` works in Rust.

## Is there more you would like to do with dyner?

Yes! Here are some things we would like to support but haven't gotten around to yet:

* Supports for arbitrary smart pointers. In particular, it'd be great if you could use `from_ref` with anything `impl Deref<Target: Trait>`, or at least any `impl Deref` that is pointer sized. Not clear how to best express or manage invoking the destructor.


