> Note: this crate is in an experimental sketch state. Please be careful if using on production environments.

---

# `nerf`

`nerf` stands for:

- No-nonsense: Correctly handle every cases(including errors, rate limits, etc.) without gotchas/`unsafe`s.
- Ergonomic: Frictionless integration into existing ecosystems such as [`tower`](https://crates.io/crates/tower) and [`hyper`](https://crates.io/crates/hyper).
- Request Framework: Provides abstractions and utility implementations for (mainly HTTP) request/response pairs.