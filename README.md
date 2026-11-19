<p align="center">
  <img src="https://diegopn.github.io/potoo-site/src/assets/images/language-interpreter-icon.svg?v=1c651ca7" alt="Potoo IDE language interpreter icon" width="120" />
</p>

<h1 align="center">AlgoBox Interpreter</h1>

<p align="center">A Rust interpreter for AlgoBox's textual French syntax.</p>

<p align="center">
  <a href="https://github.com/diegopn/potoo-algobox-interpreter/releases">Download</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/algobox/">Website</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/algobox/">User manual</a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://doc.rust-lang.org/cargo/"><img alt="Cargo" src="https://img.shields.io/badge/Cargo-DEA584?style=for-the-badge&logo=rust&logoColor=white"></a>
  <img alt="Rust library" src="https://img.shields.io/badge/Rust_library-4A86CF?style=for-the-badge">
  <img alt="CLI" src="https://img.shields.io/badge/CLI-6A4C93?style=for-the-badge">
</p>

AlgoBox is a French educational algorithm environment with structured and textual editing. This Rust package is part of the Potoo IDE ecosystem and is distributed independently from the desktop interface; it interprets the textual syntax and does not read or recreate the structured editor's internal representation. Integration with the IDE has not been implemented or verified.

## Supported language features

- `VARIABLES`, `EST_DU_TYPE NOMBRE/CHAINE/LISTE`, and the `DEBUT_ALGORITHME` / `FIN_ALGORITHME` program envelope.
- Numeric, string, Boolean, and list expressions; one-based list access; arithmetic, comparisons, `ET`, and `OU`.
- `PREND_LA_VALEUR`, `LIRE`, `AFFICHER`/`AFFICHER*`, numeric `AFFICHERCALCUL`/`AFFICHERCALCUL*`, `SI...ALORS` with optional `SINON`, `POUR...ALLANT_DE...A`, and `TANT_QUE`.
- Documented local functions with parameters, local variables, recursion, `RENVOYER`, and statement-form `APPELER_FONCTION`; numeric, trigonometric, list-statistics, probability, and factorial functions; and host-routed random, pause, drawing, and cancellation services.
- String operations `substr`, `length`, `toString`, `parseInt`, `parseFloat`, `charCodeAt`, and `String.fromCharCode`, with the zero-based positions and ASCII codes stated by the manual.
- Numeric `max`/`min` helpers and decimal-place rounding with `ALGOBOX_ARRONDIR(x,n)`.
- `F1` formula calls supplied by the host, plus host callbacks for the configured multi-argument `F2` function and loaded extension functions.

Lists keep shared identity when passed into a local function, so indexed updates through a list parameter are visible to the caller. `Host::call_local_function` supplies configured `F1`/`F2` formulas and loaded extension functions. Plotting, graph clearing, pause, and random-number generation are supplied by the host. The crate does not recreate AlgoBox's structured editor, debugger, or result window.

See [reference coverage](REFERENCE-COVERAGE.md) for the feature-by-feature mapping and host boundaries.

## Use the library

Call `parse(source)` and pass the resulting `Program` to `Interpreter::new(host).run(&program)`. Implement `Host` to provide input, output, drawing, and random integer services. Hosts can also confirm continuation after each 1,000 output operations, cancel through `should_stop`, and supply configured formula or extension functions through `call_local_function`. Runtime values and final variables are returned as public types.

## Command line

```text
cargo run --locked -- source.algobox
```

The runner reads a UTF-8 source file, takes `LIRE` values from standard input, and writes `AFFICHER` and `AFFICHERCALCUL` output to standard output. Drawing instructions return an error because the CLI has no plotting surface.

## Reference

- [AlgoBox user manual](https://www.xm1math.net/algobox/doc.html)
- [AlgoBox changelog](https://www.xm1math.net/algobox/changelog.html)
- [AlgoBox downloads and source releases](https://www.xm1math.net/algobox/download.html)
- [Potoo AlgoBox manual](https://diegopn.github.io/potoo-site/manual/algobox/)

## Support

<p align="left">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## Project policies

- [Code of conduct](CODE_OF_CONDUCT.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)
