# vitrine_derive

Derive macros for [`Vitrine`](https://github.com/charlyisidore/vitrine).

## Usage

```rust
use vitrine_derive::{FromJs, FromLua, FromRhai};

#[derive(Debug, FromJs, FromLua, FromRhai)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    use vitrine::util::eval;

    // Read value from JavaScript
    let point: Point = eval::js::from_str("({ x: 1, y: 2 })").unwrap();

    println!("point = {:?}", point);

    // Read value from Lua
    let point: Point = eval::lua::from_str("{ x = 1, y = 2 }").unwrap();

    println!("point = {:?}", point);

    // Read value from Rhai
    let point: Point = eval::rhai::from_str("#{ x: 1, y: 2 }").unwrap();

    println!("point = {:?}", point);
}
```
