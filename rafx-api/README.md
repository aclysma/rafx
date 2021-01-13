# Rafx API

Rafx API is an `unsafe` graphics API abstraction layer designed specifically for games and tools
for games. The goal is to achieve near-native performance with reduced complexity. It may be used
directly, or indirectly through other crates in rafx (such as Rafx Resources and Rafx Assets).

It is an **opinionated** API. It does not expose every possible operation a graphics API might provide.
However, the wrapped API-specific objects are exposed in an easily accessible manner.

The API does not track resource lifetimes or states (such as vulkan image layouts)
or try to enforce safe usage at compile time. Safer abstractions are available in 
rafx-resources and rafx-assets.

**Every API call is potentially unsafe.** However, the unsafe keyword is only placed on APIs that are
particularly likely to cause undefined behavior if used incorrectly.

The general shape of the API is inspired by [The Forge](https://github.com/ConfettiFX/The-Forge). It was
chosen for its modern design, multiple working backends, open development model, and track record of
shipped games.

However, it is not a 1:1 copy. There are some changes in API design, feature set, and implementation details.

### High-level API Design

A few paragraphs that cover all concepts

Lifetimes

Resource States

Structs as params

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

See [LICENSE-APACHE](../LICENSE-APACHE) and [LICENSE-MIT](../LICENSE-MIT).
