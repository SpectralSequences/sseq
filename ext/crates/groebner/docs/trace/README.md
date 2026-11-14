# Trace of a resolution

`resolution.tex` follows the `groebner` crate through a minimal resolution of $\mathbb{F}_2$ over the
Steenrod algebra, one reduction at a time.
`groebner.tex` and `groebner_p3.tex` show only the results of larger computations, at $p = 2$ and
$p = 3$. All are generated in two steps.

`groebner-trace` (this crate) runs `Resolution::sphere` one internal degree at a time, with an
observer recording which pairs, singles and relations were reduced. It then redoes each reduction
with the crate's own products, recording every step. After each degree it checks that the outcomes,
the new basis elements and the differentials agree with the program's. It also replays the
Gebauer–Möller divisibility criterion on the final leading terms, checks that it prunes as many
pairs as the program counted, and prints everything as JSON.

`render.py` turns that JSON into LaTeX, filling in `template.tex`. It names each generator after the
class of Ext it is dual to, computing the products with $h_i$ from the differentials. Indecomposable
classes other than the $h_i$ are named from a table in the script by their bidegree, and the script
fails if it meets a generator it cannot name. At $p = 3$, products with $a_0$ (dual to $Q_0$) and
$h_i$ (dual to $\mathcal{P}^{3^i}$) are found the same way, and the classes that are not such
products and not $a_1$ or $b_i$ are labelled $x_{n,s}$ by their bidegree.

```sh
cd ext/crates/groebner/docs/trace
# The full account, step by step.
cargo run --release -- 6 10 > trace.json        # s_max, n_max
python3 render.py trace.json template.tex ../resolution.tex
# Only the results: --summary computes in one go and skips the step-by-step replay.
cargo run --release -- --summary 10 20 > trace.json
python3 render.py --summary trace.json template.tex ../groebner.tex
# The same at p = 3.
cargo run --release --features concurrent -- --summary --prime 3 25 100 > trace.json
python3 render.py --summary trace.json template_p3.tex ../groebner_p3.tex
cd .. && pdflatex resolution.tex && pdflatex resolution.tex   # likewise the others
```

The step-by-step account is only supported at $p = 2$.
