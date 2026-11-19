# AlgoBox reference coverage

This crate follows the textual AlgoBox syntax described in the official user manual and changelog. It does not parse the structured editor's saved tree or recreate its graphical interface.

| Reference area | Crate behavior | Boundary |
| --- | --- | --- |
| Variables and expressions | `NOMBRE`, `CHAINE`, and numeric `LISTE`; arithmetic, comparisons, conditions, `ET`/`OU`, and numeric expressions. `Math.PI`, documented numeric functions, `max`/`min`, and `ALGOBOX_ARRONDIR(x,n)` are available. | Boolean values are internal to condition evaluation; declarations follow the manual's three variable types. Automatic display rounding remains an application preference. |
| Control and routines | Assignments, `LIRE`, `AFFICHER`/`AFFICHER*`, numeric `AFFICHERCALCUL`/`AFFICHERCALCUL*`, `SI`/`SINON`, `POUR` with real numeric bounds and automatic `+1`, `TANT_QUE`, `PAUSE`, local functions, local variables, recursion, `RENVOYER`, and statement-form `APPELER_FONCTION`. | Each loop is limited to 500,000 iterations and all loops together to 5,000,000, matching the documented defaults. After 1,000 output operations, `Host::confirm_continue_after_output_limit` asks the host whether to continue. Local-function calls retain a 256-level safety limit because they use Rust call frames. `Host::should_stop` lets an embedding application cancel execution. |
| Lists | One-based list indexing, colon-separated list literals and input, shared list identity across assignments and function parameters, and inclusive range functions for sum, mean, variance, standard deviation, median, quartiles, minimum/maximum, and their positions. | `QUARTILE1`/`QUARTILE3` use the median of the lower/upper half. The `*_BIS` forms use the nearest ordered rank for 25%/75%. Quartiles require at least four values, as documented. |
| Numbers and probability | `sqrt`, `pow`, `floor`, trigonometric and inverse-trigonometric functions, `exp`, `log`, `abs`, `round`, `%`, `random`, `ALGOBOX_ALEA_ENT`, binomial coefficients/probabilities, normal CDF/inverse, and factorial for integers from 0 through 69. | Random values are provided by `Host`; hosts may override the real-valued generator. The normal CDF and inverse use a floating-point approximation. |
| Strings | `substr`, `length`, `toString`, `parseInt`, `parseFloat`, `charCodeAt`, and `String.fromCharCode`; string positions start at zero and character-code operations use ASCII. | `substr` and `length` count Unicode scalar values. Non-ASCII values have no ASCII code and return an error. Invalid numeric prefixes produce the JavaScript-compatible `NaN` value. |
| Configured functions and extensions | `F1` accepts one numeric argument through `Host::call_local_function`; configured `F2` and loaded extension calls can use the same host callback. | The formula editor and arbitrary JavaScript extension runtime are not embedded in the crate. The host supplies those configured functions. |
| Drawing and execution UI | `TRACER_POINT` and `TRACER_SEGMENT` accept the documented coordinate-pair syntax and `Rouge`, `Vert`, `Bleu`, or `Blanc` variants; `EFFACER_GRAPHIQUE` clears the drawing via `Host`. Pause, input, and output also route through the host, and `Host::should_stop` supports cancellation. | The host owns the drawing surface, interactive pause, step-by-step execution, and display confirmation after many outputs. `Blanc` is forwarded as the source's erase-color command; the host applies it to its plot. The CLI does not provide a plot window or debugger. |

## Sources

- [Official AlgoBox user manual](https://www.xm1math.net/algobox/doc.html)
- [Official AlgoBox changelog](https://www.xm1math.net/algobox/changelog.html)
- [Official AlgoBox source releases](https://www.xm1math.net/algobox/download.html)

This table records implementation and host boundaries. It is not a claim of compatibility with every AlgoBox release or with its visual editor.
