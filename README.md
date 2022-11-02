# da-simulator

`da-simulator` is a highly parallel simulator capable of running arbitrary distributed algorithms of various models of computation (PN, LOCAL, CONGEST) on networks constructed from arbitrary graphs.

## Features

- Highly versatile
  - Supports algorithms in PN, LOCAL and CONGEST models
  - Algorithms follow the [formal distributed algorithm definition](https://jukkasuomela.fi/da2020/da2020-03.pdf) (Section 3.3)
- Network output in [Graphviz DOT](https://graphviz.org/doc/info/lang.html) format
- Deadlock detection and prevention
- Communication round limiting
- Thread-per-node architecture
- (Relatively) easy to debug™
- Written in 100% safe Rust

## Usage

> **Note:** `da-simulator` currently requires nightly Rust, see [#63063](https://github.com/rust-lang/rust/issues/63063). Hint: you can switch to the nighly channel by running `rustup default nightly`.

Select (or define) a network and an algorithm to simulate on that network in `main.rs`, then just run compile and run the application:

```shell
cargo run --release
```

## Authors

- Dennis Marttinen ([@twelho](https://github.com/twelho))

## License

[Mozilla Public License Version 2.0](https://mozilla.org/MPL/2.0/) ([LICENSE](./LICENSE))

## Acknowledgements

Special thanks to [Jukka Suomela](https://jukkasuomela.fi/) for the amazing [Distributed Algorithms course](https://jukkasuomela.fi/da2020/) for which this simulator has been developed for and from where the algorithms it simulates are sourced from.
