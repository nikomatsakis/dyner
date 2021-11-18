# Welcome

The `dyner` crate is an experimental crate that is exploring the "outer reaches" of how we can improve trait objects (`dyn` types) in Rust.

`dyner` makes it possible to use dynamic dispatch on traits that...

* ...have async functions.
* ...use `impl Trait` in argument or return position, so long as `Trait` is also processed with `dyner`.
* ...have by-value `self` methods.

More dyner features:

* No need to worry about `?Sized`! 🧑‍🍳 😘

## Quick start

To give you an idea for how dyner works, consider this pair of traits:

```rust
#[dyner]
trait Screen {
    fn put(&mut self, ch: char, x: u32, y: u32);
}

#[dyner]
trait Draw {
    fn draw(&self, screen: &mut impl Screen);
}
```

You can't use ordinary `dyn` types with these traits. But because we added the `#[dyner]` annotation, we have access to two types, `DynScreen` and `DynDraw`, that we can use to get dynamic dispatch. For example, we might collect a vector of "drawable things":

```rust
fn make_drawables() -> Vec<DynDraw<'static>> {
    let r = Rectangle::new(0, 0, 10, 10);
    let c = Circle::new(22, 44, 66);
    vec![DynDraw::new(r), DynDraw::new(c)]
}
```

The `DynDraw<'static>` type here means "some `Draw` object that has no references". You could also write a routine that draws these things onto a screen:

```rust
fn draw_all(draws: &[DynDraw<'_>], screen: &mut impl Screen) {
    for draw in draws {
        draws.draw(screen);
    }
}
```

Note that this version of `draw_all` used an `impl Screen`, and hence we would create a distinct version of this method for every kind of screen. To avoid that, maybe we want to use dynamic dispatch there too. No problem, just change `impl Screen` to `DynScreen<'_>`, and everything still works:

```rust
fn draw_all(draws: &[DynDraw<'_>], screen: &mut DynScreen<'_>) {
    for draw in draws {
        draws.draw(screen);
    }
}
```

## Objects from references

In the previous examples, we used `DynDraw::new` to construct an object; the `new` method takes ownership of the data in the object. But sometimes we just have an `&impl Trait` and we'd like to get dynamic dispatch from that. You can use the `from_ref` method to do that. In this code, the `draw_four` method is implemented with `impl Trait`, but it calls into the `draw_two` method, which is implemented with dynamic dispatch:

```rust
fn draw_four(draw: &impl Draw, screen: &mut impl Screen) {
    let draw_ref = DynDraw::from_ref(draw);
    let screen_mut = DynScreen:from_mut(screen);
    draw_two(&draw_ref, screen);
    draw_two(&draw_ref, screen);
}

fn draw_two(draw: &DynDraw<'_>, screen: &mut DynScreen<'_>) {
    draw.draw(screen);
    draw.draw(screen);
}
```

## No std? No problem.

By default, `DynDraw` depends on std, since `DynDraw::new` needs to allocate a `Box` behind the scenes. If you opt out from the default features, you will lose access to `DynDraw::new`, but you can still use `DynDraw::from_ref` and `DynDraw::from_mut`.

