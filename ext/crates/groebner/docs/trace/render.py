"""Render the JSON written by the trace driver as a LaTeX account of the resolution.

Usage: python3 render.py [--summary] trace.json template.tex out.tex

With --summary, the step-by-step account is left out and only the results are shown. The
step-by-step account is only supported at p = 2. Use template_p3.tex at p = 3.
"""

import argparse
import json

parser = argparse.ArgumentParser()
parser.add_argument("--summary", action="store_true", help="show only the results")
parser.add_argument("trace")
parser.add_argument("template")
parser.add_argument("out")
args = parser.parse_args()

d = json.load(open(args.trace))
S_MAX, N_MAX = d["s_max"], d["n_max"]
P = d.get("p", 2)
assert P == 2 or args.summary, "the step-by-step account is only supported at p = 2"


if P == 2:
    # Indecomposable classes other than the h_i, by (s, stem).
    INDEC = {(3, 8): "c_0", (5, 9): "Ph_1", (5, 11): "Ph_2", (4, 14): "d_0", (4, 17): "e_0",
             (4, 18): "f_0", (3, 19): "c_1", (4, 20): "g", (7, 16): "Pc_0", (9, 17): "P^2h_1",
             (9, 19): "P^2h_2", (8, 22): "Pd_0"}
    # The classes whose products are read off the differentials: h_i is dual to Sq^{2^i}.
    H = [f"h_{i}" for i in range(8)]
else:
    INDEC = {(2, 2 * P + 1): "a_1"}
    INDEC.update({(2, P ** (i + 1) * (2 * P - 2) - 2): f"b_{i}" for i in range(4)})
    # a_0 is dual to Q_0, and h_i to P^{p^i}.
    H = ["a_0"] + [f"h_{i}" for i in range(8)]


def dual_index(m):
    """The index in H of the class dual to the Milnor basis element m, or None."""
    r = [x for x in m["r"]]
    while r and r[-1] == 0:
        r.pop()
    if P == 2:
        if len(r) == 1 and r[0] & (r[0] - 1) == 0:
            return r[0].bit_length() - 1
        return None
    if m["e"] == 1 and not r:
        return 0
    if m["e"] == 0 and len(r) == 1:
        i = 0
        while P**i < r[0]:
            i += 1
        if P**i == r[0]:
            return i + 1
    return None


def fmt(f):
    """The name of the product of classes with exponents f."""
    out = ""
    for k in H + sorted(x for x in f if x not in H):
        e = f.get(k, 0)
        if e:
            out += k if e == 1 else (f"{k}^{{{e}}}" if "^" not in k else f"({k})^{{{e}}}")
    return out


def name_generators(gens):
    """Name the generators: names[s][i], products[(s, i, k)] = targets of H[k] times class (s, i),
    and the list of (s, stem, i) that are neither products nor in INDEC."""
    names = [[None] * len(gl) for gl in gens]
    facs = [[None] * len(gl) for gl in gens]
    names[0][0], facs[0][0] = "1", {}
    # H[k] * x_g is non-zero on g' exactly when the dual of H[k] applied to g occurs in d g'.
    prod = {}
    for s in range(1, len(gens)):
        for j, g in enumerate(gens[s]):
            for gen, m, c in g["d"]:
                k = dual_index(m)
                if k is not None:
                    prod.setdefault((s - 1, gen, k), []).append(j)
    unnamed = []
    taken = set()
    for s in range(1, len(gens)):
        for j, g in enumerate(gens[s]):
            stem = g["t"] - s
            for k in range(len(H)):
                srcs = [x for x in range(len(gens[s - 1])) if prod.get((s - 1, x, k)) == [j]]
                srcs = [x for x in srcs if facs[s - 1][x] is not None]
                if srcs:
                    f = dict(facs[s - 1][srcs[0]])
                    f[H[k]] = f.get(H[k], 0) + 1
                    names[s][j], facs[s][j] = fmt(f), f
                    break
            else:
                name = INDEC.get((s, stem))
                if name is not None and name not in taken:
                    names[s][j], facs[s][j] = name, {name: 1}
                    taken.add(name)
                else:
                    unnamed.append((s, stem, j))
    if P != 2:
        # Label the remaining indecomposables by their bidegree.
        for s, stem, j in unnamed:
            same = [x for x in unnamed if x[:2] == (s, stem)]
            suffix = "" if len(same) == 1 else "'" * same.index((s, stem, j))
            names[s][j] = f"x_{{{stem},{s}}}{suffix}"
        unnamed = []
    return names, prod, unnamed


gens = d["generators"]


NAMES, PRODUCTS, UNNAMED = name_generators(gens)
assert not UNNAMED, f"no names for {UNNAMED}"


def gname(s, i):
    if s == 0:
        return r"\iota"
    return f"[{NAMES[s][i]}]"


def qpart(m):
    """The exterior part Q(E) of m."""
    return "".join(f"Q_{{{k}}}" for k in range(16) if m["e"] >> k & 1)


def sq(m):
    """The Milnor basis element m, as Sq(R) at p = 2 and Q(E)P(R) at odd primes."""
    R = list(m["r"])
    while R and R[-1] == 0:
        R.pop()
    op = r"\Sq" if P == 2 else r"\P"
    if not R:
        rp = ""
    elif len(R) == 1:
        rp = rf"{op}^{{{R[0]}}}"
    else:
        rp = rf"{op}({','.join(map(str, R))})"
    return qpart(m) + rp


def xmono(m):
    """The monomial of m in the associated graded algebra."""
    R = m["r"]
    parts = [qpart(m)] if m["e"] else []
    # Positions in increasing degree: by j = i + n, then i descending.
    for j in range(1, 12):
        for i in range(j - 1, -1, -1):
            n = j - i
            if n <= len(R):
                digit = R[n - 1] // P**i % P
                if digit:
                    parts.append(f"x_{{{i}{n}}}" + (f"^{digit}" if digit > 1 else ""))
    return "".join(parts) if parts else "1"


def coef(c, first):
    """The sign and coefficient c in front of a term, with c in 1..p-1 printed as -1 for p - 1."""
    if c == 1:
        return "" if first else " + "
    if c == P - 1:
        return "-" if first else " - "
    return f"{c}" if first else f" + {c}"


def term(s, t):
    g, m = t[0], t[1]
    return f"{sq(m)}{gname(s, g)}"


def poly(s, x):
    if not x:
        return "0"
    return "".join(coef(t[2] if len(t) > 2 else 1, i == 0) + term(s, t) for i, t in enumerate(x))


def lt(s, t):
    m = xmono(t[1])
    return (m + r"\," if m != "1" else "") + gname(s, t[0])


def display(s, e):
    """A displayed element, broken into its components when it is too wide for one line."""
    short = elem(s, e)
    if len(short) <= 140:
        return rf"\[{short}.\]"
    return (
        r"\begin{quote}\raggedright"
        + "\n"
        + rf"first component $x_1 = {poly(s, e['x1'])}$,\\"
        + "\n"
        + rf"second component $x_2 = {poly(s + 1, e['x2'])}$."
        + "\n"
        + r"\end{quote}"
    )


def elem(s, e):
    return rf"\bigl({poly(s, e['x1'])} \;\big|\; {poly(s + 1, e['x2'])}\bigr)"


levels = d["levels"]


def b(s, k):
    return f"b^{{{s}}}_{{{k}}}"


def LT(s, k):
    return lt(s, levels[s][k]["x1"][0])


def steps_list(s, steps, comp, out):
    """comp: 'x1' reduces the first component of level s (shown with second component);
    'x2' reduces a cycle of C_{s+1} modulo level s+1."""
    out.append(r"\begin{itemize}")
    for st in steps:
        tm = st["term"]
        k = st["k"]
        q = st["quotient"]
        if comp == "x1":
            prod = elem(s, {"x1": st["prod_x1"], "x2": st["prod_x2"]})
            after = elem(s, {"x1": st["after_x1"], "x2": st["after_x2"]})
            out.append(
                rf"\item The term ${term(s, tm)}$ has leading monomial ${lt(s, tm)}$, divisible by "
                rf"$\LT({b(s, k)})$ with quotient ${xmono(q)}$. Adding ${sq(q) or '1'}\cdot {b(s, k)} = {prod}$ "
                rf"leaves ${after}$."
            )
        else:
            s1 = s + 1
            after = poly(s1, st["after_x1"])
            out.append(
                rf"\item The term ${term(s1, tm)}$ has leading monomial ${lt(s1, tm)}$, divisible by "
                rf"$\LT({b(s1, k)})$ with quotient ${xmono(q)}$. Adding "
                rf"${sq(q) + r'\cdot ' if sq(q) else ''}\pi_1({b(s1, k)}) = {poly(s1, st['prod_x1'])}$ leaves ${after}$."
            )
    out.append(r"\end{itemize}")


def kind_word(it):
    return {"single": "single", "pair": "pair"}[it["kind"]]


def header(it):
    s = it["s"]
    if it["kind"] == "relation":
        r = it["start"]["x2"][0]
        return (
            rf"\paragraph{{Relation ${term(0, r)} = 0$.}} The module relation enters as the element "
            rf"$(0 \mid {term(0, r)})$ of level $-1$, so its $C_0$ component is a candidate for $d$ of "
            r"a new generator. It is reduced modulo the image $\operatorname{im}(d\colon C_1\to C_0)$, "
            r"i.e.\ the first components of level $0$."
        )
    i2 = it["i2"]
    if it["kind"] == "single":
        m = it["m2"]
        return (
            rf"\paragraph{{Level ${s}$, single ${xmono(m)}\cdot {b(s, i2)}$.}} Since ${xmono(m)}$ divides "
            rf"$\LT({b(s, i2)}) = {LT(s, i2)}$ and $x_{{in}}^2 = 0$ in $E^0\A$, the leading term of "
            rf"${sq(m)}\cdot {b(s, i2)}$ is not the product of the leading terms, and the product has to "
            rf"be reduced:" + "\n" + display(s, it['start'])
        )
    i1 = it["i1"]
    m1, m2 = it["m1"], it["m2"]
    lead = levels[s][i1]["x1"][0]
    lcm = it["lcm"]
    return (
        rf"\paragraph{{Level ${s}$, pair $({b(s, i1)}, {b(s, i2)})$.}} The leading terms "
        rf"$\LT({b(s, i1)}) = {LT(s, i1)}$ and $\LT({b(s, i2)}) = {LT(s, i2)}$ have least common multiple "
        rf"${xmono(lcm)}\,{gname(s, lead[0])}$, with cofactors ${xmono(m1)}$ and ${xmono(m2)}$. The "
        rf"S-element ${sq(m1)}\cdot {b(s, i1)} + {sq(m2)}\cdot {b(s, i2)}$, in which the leading terms "
        rf"cancel, is" + "\n" + display(s, it['start'])
    )


def item_tex(it, cyc=None):
    s = it["s"]
    out = [header(it)]
    x1 = it["start"]["x1"]
    x2 = it["start"]["x2"]
    if s >= 0 and x1:
        if it["steps1"]:
            out.append(rf"Reduce the first component modulo level ${s}$:")
            steps_list(s, it["steps1"], "x1", out)
            x1 = it["steps1"][-1]["after_x1"]
            x2 = it["steps1"][-1]["after_x2"]
        else:
            out.append(rf"No term of the first component is divisible by a leading term of level ${s}$.")
    if s >= 0 and not x1:
        out.append(
            rf"The first component is zero, so the second component $z = {poly(s + 1, x2)}$ is a cycle "
            rf"in $C_{{{s + 1}}}$."
        )
    if s + 1 >= S_MAX:
        if not x1:
            out.append(rf"There is no level ${s + 1}$ (we stop at $s = {S_MAX}$).")
    elif it["steps2"]:
        if x1:
            out.append(
                rf"The second component is reduced modulo the first components of level ${s + 1}$, "
                r"which changes it by a boundary:"
            )
        else:
            out.append(
                rf"Reduce it modulo $\operatorname{{im}}(d\colon C_{{{s + 2}}}\to C_{{{s + 1}}})$, the "
                rf"first components of level ${s + 1}$:"
            )
        steps_list(s, it["steps2"], "x2", out)
        x2 = it["steps2"][-1]["after_x1"]
    elif not x1 and s >= 0:
        out.append(rf"No term of it is divisible by a leading term of level ${s + 1}$ from earlier degrees.")
    elif s < 0:
        out.append(r"No term of it is divisible by a leading term of level $0$ from earlier degrees.")
    for tr in it["tri1"]:
        k = tr["k"]
        out.append(
            rf"Its first component contains $\LT({b(s, k)})$ of the element ${b(s, k)}$ just found in "
            rf"this degree; adding it gives ${elem(s, tr['after'])}$."
        )
        x1, x2 = tr["after"]["x1"], tr["after"]["x2"]
    for tr in it["tri2"]:
        k = tr["k"]
        out.append(
            rf"The second component contains the leading term of ${b(s + 1, k)}$, found in this degree; "
            rf"adding its first component leaves ${poly(s + 1, tr['after'])}$."
        )
        x2 = tr["after"]
    o = it["outcome"]
    if o == "element":
        k = it["index"]
        out.append(
            rf"\textbf{{Outcome: new basis element}} ${b(s, k)} = {elem(s, it['result'])}$ of level "
            rf"${s}$, with leading term ${LT(s, k)}$."
        )
    elif o == "useless":
        if it["tri2"]:
            k = it["tri2"][-1]["k"]
            src = poly(s + 2, levels[s + 1][k]["x2"])
            if len(it["tri2"]) == 1:
                out.append(
                    rf"\textbf{{Outcome: useless.}} The cycle is the first component of ${b(s + 1, k)}$, "
                    rf"found one level up earlier in this degree, so it is the boundary $d\bigl({src}\bigr)$. "
                    r"Both components now vanish, and the reduction contributed nothing."
                )
            else:
                out.append(
                    r"\textbf{Outcome: useless.} The cycle was a sum of first components of elements found "
                    r"one level up earlier in this degree, hence a boundary. Both components now vanish, "
                    r"and the reduction contributed nothing."
                )
        else:
            out.append(r"\textbf{Outcome: useless.} Both components vanish, and the reduction contributed nothing.")
    elif o == "cycle":
        if s >= 0:
            out.append(
                rf"The cycle ${poly(s + 1, x2)}$ is set aside: cycles are tested once all results of "
                rf"level ${s}$ in this degree are sorted."
            )
        else:
            out.append(cycle_outcome(s, cyc, inline=True))
    return "\n".join(out)


def cycle_outcome(s, c, inline=False):
    out = []
    it = items[c["item"]]
    z = it["cycle"]
    s1 = s + 1
    for tr in c["tri3"]:
        k = tr["k"]
        out.append(
            rf"The cycle ${poly(s1, z)}$ contains the leading term of ${b(s1, k)}$, found in this degree "
            rf"from an earlier cycle; adding its first component leaves ${poly(s1, tr['after'])}$."
        )
        z = tr["after"]
    o = c["outcome"]
    if o == "boundary":
        k = c["tri3"][-1]["k"]
        out.append(
            rf"\textbf{{Outcome: boundary.}} The cycle is $d\bigl({poly(s + 2, levels[s1][k]['x2'])}\bigr)$, "
            r"a boundary found only now: the generator it bounds was created from another cycle "
            r"earlier in this degree. It gives nothing new, but unlike a useless reduction, it was "
            r"only recognised after both components had been reduced."
        )
    elif o == "beyond":
        out.append(
            rf"\textbf{{Outcome: beyond.}} The cycle ${poly(s1, z)}$ would give a generator of "
            rf"$C_{{{s + 2}}}$, beyond $s = {S_MAX}$, so it is dropped."
        )
    else:
        s2, i = c["gen"]
        g = gname(s2, i)
        what = "relation" if inline else "cycle"
        out.append(
            rf"\textbf{{Outcome: new generator.}} The {what} ${poly(s1, z)}$ is not a boundary, so "
            rf"$C_{{{s2}}}$ gets a generator ${g}$ in degree ${gens[s2][i]['t']}$ with "
            rf"$d{g} = {poly(s1, z)}$."
        )
        if "index" in c:
            k = c["index"]
            out.append(
                rf"Level ${s1}$ gets the element ${b(s1, k)} = \bigl({poly(s1, z)} \;\big|\; {g}\bigr)$, "
                rf"with leading term ${lt(s1, z[0])}$."
            )
    return "\n".join(out)


def pruned_tex(pr):
    s, i, n, j = pr["s"], pr["i"], pr["n"], pr["by"]
    return (
        rf"\paragraph{{Pruned: level ${s}$, pair $({b(s, i)}, {b(s, n)})$.}} Its cofactors are "
        rf"${xmono(pr['m1'])}$ and ${xmono(pr['m2'])}$. When ${b(s, n)}$ joined level ${s}$, the pair "
        rf"$({b(s, j)}, {b(s, n)})$ of degree ${pr['by_t']}$ had the cofactor ${xmono(pr['by_m2'])}$ on "
        rf"${b(s, n)}$, which divides ${xmono(pr['m2'])}$. So the syzygy of $({b(s, i)}, {b(s, n)})$ is a "
        rf"multiple of that of $({b(s, j)}, {b(s, n)})$ plus a syzygy between ${b(s, i)}$ and "
        rf"${b(s, j)}$, and the Gebauer--M\"oller criterion discards it: the pair is never formed."
    )


def and_join(xs):
    """Join a list of phrases with commas and a final "and"."""
    return xs[0] if len(xs) == 1 else ", ".join(xs[:-1]) + " and " + xs[-1]


NUM = {1: "one", 2: "two", 3: "three", 4: "four", 5: "five", 6: "six"}

body = []
for deg in d["degrees"]:
    t = deg["t"]
    items = deg["items"]
    if not items:
        continue
    body.append(rf"\subsection*{{Degree $t = {t}$}}")
    dropped = [s for s in range(-1, S_MAX) if s not in deg["active"]]
    if dropped and dropped != [-1]:
        lv = [s for s in dropped if s >= 0]
        if lv:
            names = ", ".join(f"${s}$" for s in lv)
            last = lv[-1]
            body.append(
                rf"\noindent Level{'s' if len(lv) > 1 else ''} {names} {'are' if len(lv) > 1 else 'is'} no "
                rf"longer needed: a cycle found at level $s$ in this degree would be a generator of "
                rf"$C_{{s+2}}$ in stem ${t}-s-2 > {N_MAX}$ for $s \le {last}$, so "
                rf"{'their' if len(lv) > 1 else 'its'} pairs are not formed."
            )
    # Work summary.
    work = []
    rel = [it for it in items if it.get("kind") == "relation"]
    if rel:
        r = rel[0]["start"]["x2"][0]
        work.append(f"the relation ${term(0, r)}$")
    for s in range(S_MAX):
        ks = [it["kind"] for it in items if it.get("s") == s and it.get("kind") in ("single", "pair")]
        if ks:
            ns, np_ = ks.count("single"), ks.count("pair")
            parts = []
            if ns:
                parts.append(f"{NUM.get(ns, ns)} single{'s' if ns > 1 else ''}")
            if np_:
                parts.append(f"{NUM.get(np_, np_)} pair{'s' if np_ > 1 else ''}")
            work.append(f"level ${s}$: " + ", ".join(parts))
    body.append(
        r"\noindent The work of this degree: " + "; ".join(work) + ". All of them are reduced "
        r"independently (in parallel in the program), and the results are then sorted from the top "
        r"level down."
    )
    for pr in [p for p in d["pruned"] if p["t"] == t]:
        body.append(pruned_tex(pr))
    body.append(r"\medskip")
    cyc_by_item = {}
    for it in items:
        if it.get("kind") == "cycles":
            for c in it["cycles"]:
                cyc_by_item[c["item"]] = c
    new_el, new_gen = {}, []
    for idx, it in enumerate(items):
        if it["kind"] == "cycles":
            s = it["s"]
            if s < 0:
                continue
            n = len(it["cycles"])
            body.append(
                rf"\paragraph{{The cycles of level ${s}$.}} There {'is one' if n == 1 else 'are ' + NUM.get(n, str(n))}."
            )
            if n == 1:
                body.append(cycle_outcome(s, it["cycles"][0]))
            else:
                body.append(r"\begin{enumerate}")
                for c in it["cycles"]:
                    body.append(r"\item " + cycle_outcome(s, c))
                body.append(r"\end{enumerate}")
            body.append(r"\medskip")
        else:
            body.append(item_tex(it, cyc_by_item.get(idx)))
            body.append(r"\medskip")
        if it["kind"] != "cycles" and it["outcome"] == "element":
            new_el.setdefault(it["s"], []).append(it["index"])
    for it in items:
        if it["kind"] == "cycles":
            for c in it["cycles"]:
                if c["outcome"] == "generator":
                    s2, i = c["gen"]
                    new_gen.append((s2, i))
                    if "index" in c:
                        new_el.setdefault(s2 - 1, []).append(c["index"])
    if new_el:
        ne = "; ".join(
            f"level ${s}$: " + ", ".join(f"${b(s, k)}$" for k in sorted(v)) for s, v in sorted(new_el.items())
        )
        ne = f"New basis elements: {ne}."
    else:
        ne = "No new basis elements."
    if new_gen:
        ng = "New generators: " + ", ".join(f"${gname(s, i)}\\in C_{{{s}}}$" for s, i in sorted(new_gen, key=lambda x: (x[0], x[1]))) + "."
    else:
        ng = "No new generators."
    body.append(rf"\par\noindent\emph{{End of degree {t}.}} {ne} {ng}")

# Results.
res = []
res.append(r"\paragraph{Generators and differentials.}")
res.append(r"\begin{center}\small")
res.append(r"\begin{longtable}{lccc>{\raggedright\arraybackslash}p{0.62\linewidth}}")
res.append(r"\toprule")
res.append(r"generator & $s$ & $t$ & $t-s$ & differential \\")
res.append(r"\midrule\endhead")
rows = []
for s, gl in enumerate(gens):
    for i, g in enumerate(gl):
        rows.append((g["t"], s, i, g))
rows.sort()
for t, s, i, g in rows:
    dd = "---" if s == 0 else f"${poly(s - 1, g['d'])}$"
    res.append(rf"${gname(s, i)}$ & {s} & {t} & {t - s} & {dd} \\")
res.append(r"\bottomrule")
res.append(r"\end{longtable}")
res.append(r"\end{center}")
res.append(
    r"Since the resolution is minimal, the generators are a basis of $\Ext_\A^{s,t}(\F, \F)$ in this range:"
)


def chart_cell(s, n):
    names = [f"${NAMES[s][j] if s else '1'}$" for j, g in enumerate(gens[s]) if g["t"] - s == n]
    if len(names) > 1:
        return r"\shortstack{" + r"\\ ".join(names) + "}"
    return "".join(names)


# The number of classes of H whose products the chart draws: h_0, h_1, h_2 at p = 2, and a_0, h_0
# at odd primes, where the lines for h_1 run across eleven stems and hide the rest.
CHART_LINES = 3 if P == 2 else 2


def tikz_chart():
    """An Adams chart: a dot per generator, and lines for the products with the first classes
    of H."""
    xs, ys = 16.0 / (N_MAX + 1), min(0.45, 9.0 / (S_MAX + 1))
    pos = {}
    for s in range(len(gens)):
        by_stem = {}
        for j, g in enumerate(gens[s]):
            by_stem.setdefault(g["t"] - s, []).append(j)
        for n, js in by_stem.items():
            for k, j in enumerate(js):
                pos[(s, j)] = ((n + 0.3 * (k - (len(js) - 1) / 2)) * xs, s * ys)
    out = [r"\begin{center}", r"\begin{tikzpicture}"]
    out.append(rf"\draw[gray!40, very thin] (0,0) grid[xstep={5 * xs:.4f}, ystep={5 * ys:.4f}] ({(N_MAX + 0.5) * xs:.3f},{(S_MAX + 0.5) * ys:.3f});")
    for n in range(0, N_MAX + 1, 10):
        out.append(rf"\node[below, font=\tiny] at ({n * xs:.3f},0) {{{n}}};")
    for s in range(0, S_MAX + 1, 5):
        out.append(rf"\node[left, font=\tiny] at (0,{s * ys:.3f}) {{{s}}};")
    for (s, j, k), targets in sorted(PRODUCTS.items()):
        if k >= CHART_LINES or (s, j) not in pos:
            continue
        for t in targets:
            if (s + 1, t) in pos:
                (x0, y0), (x1, y1) = pos[(s, j)], pos[(s + 1, t)]
                out.append(rf"\draw[very thin] ({x0:.3f},{y0:.3f}) -- ({x1:.3f},{y1:.3f});")
    for (x, y) in pos.values():
        out.append(rf"\fill ({x:.3f},{y:.3f}) circle (0.9pt);")
    out += [r"\end{tikzpicture}", r"\end{center}"]
    return out


res.append(r"\begin{center}\footnotesize\setlength{\tabcolsep}{3pt}")
if N_MAX > 12 and S_MAX > 12:
    # Too large for a table: a chart, and the classes listed by stem.
    res.pop()
    res.pop()
    res += tikz_chart()
    res.append(
        "The lines are the products with "
        + and_join([f"${h}$" for h in H[:CHART_LINES]])
        + r", read off the differentials as for the names. The classes, by stem:"
    )
    res.append(r"\begin{center}\footnotesize")
    res.append(r"\begin{longtable}{r>{\raggedright\arraybackslash}p{0.85\linewidth}}")
    res.append(r"$t-s$ & classes, with $s$ in parentheses \\")
    res.append(r"\midrule\endhead")
    for n in range(N_MAX + 1):
        cls = [
            f"${NAMES[s][j] if s else '1'}$\\,({s})"
            for s in range(len(gens))
            for j, g in enumerate(gens[s])
            if g["t"] - s == n
        ]
        if cls:
            res.append(f"{n} & " + ", ".join(cls) + r" \\")
    res.append(r"\end{longtable}")
    res.append(r"\end{center}")
elif N_MAX <= 12:
    res.append(r"\begin{tabular}{c|" + "c" * (N_MAX + 1) + "}")
    res.append(r"$s \backslash t-s$ & " + " & ".join(map(str, range(N_MAX + 1))) + r" \\")
    res.append(r"\midrule")
    for s in range(S_MAX, -1, -1):
        res.append(f"${s}$ & " + " & ".join(chart_cell(s, n) for n in range(N_MAX + 1)) + r" \\")
else:
    # Too many stems for the width of a page: one row per stem instead.
    res.append(r"\begin{tabular}{c|" + "c" * (S_MAX + 1) + "}")
    res.append(r"$t-s \backslash s$ & " + " & ".join(map(str, range(S_MAX + 1))) + r" \\")
    res.append(r"\midrule")
    for n in range(N_MAX + 1):
        res.append(f"${n}$ & " + " & ".join(chart_cell(s, n) for s in range(S_MAX + 1)) + r" \\")
if not (N_MAX > 12 and S_MAX > 12):
    res.append(r"\end{tabular}")
    res.append(r"\end{center}")
GB = []
GB.append(r"\paragraph{The Gröbner bases.}")
GB.append(r"\begin{center}\scriptsize")
GB.append(r"\begin{longtable}{lcl>{\raggedright\arraybackslash}p{0.35\linewidth}>{\raggedright\arraybackslash}p{0.26\linewidth}}")
GB.append(r"\toprule")
GB.append(r"element & $t$ & leading term & first component $x_1$ & second component $x_2$ \\")
GB.append(r"\midrule\endhead")


for s, lv in enumerate(levels):
    for k, e in enumerate(lv):
        GB.append(
            rf"${b(s, k)}$ & {e['t']} & ${lt(s, e['x1'][0])}$ & ${poly(s, e['x1'])}$ & ${poly(s + 1, e['x2'])}$ \\"
        )
GB.append(r"\bottomrule")
GB.append(r"\end{longtable}")
GB.append(r"\end{center}")

ST = []
ST.append(r"\paragraph{Counters.}")
ST.append(r"The statistics the program keeps per level ($s = -1$ are the module relations):")
ST.append(r"\begin{center}")
ST.append(r"\footnotesize\setlength{\tabcolsep}{3.5pt}")
ST.append(r"\begin{tabular}{rrrrrrrrrrrr}")
ST.append(r"\toprule")
ST.append(r"$s$ & formed & divisible & chain & singles & reduced & useless & elements & boundary & gens & beyond & steps \\")
ST.append(r"\midrule")
tot = None
for st in d["stats"]:
    lab = "all" if st["s"] == 99 else str(st["s"])
    keys = ["formed", "divisible", "chain", "singles", "reduced", "useless", "elements", "boundary", "gens", "beyond", "steps"]
    ST.append(lab + " & " + " & ".join(str(st[k]) for k in keys) + r" \\")
    if st["s"] == 99:
        tot = st
ST.append(r"\bottomrule")
ST.append(r"\end{tabular}")
ST.append(r"\end{center}")

boundaries = [(t, s) for s, t, n in d["boundaries"] for _ in range(n)]
hl = []
pruned = d["pruned"]
if len(pruned) > 8:
    per_level = {}
    for p in pruned:
        per_level[p["s"]] = per_level.get(p["s"], 0) + 1
    hl.append(
        rf"The Gebauer--M\"oller divisibility criterion prunes {len(pruned)} pairs: "
        + and_join([f"{n} at level ${s}$" for s, n in sorted(per_level.items())])
        + "."
    )
elif pruned:
    hl.append(
        r"The Gebauer--M\"oller divisibility criterion prunes "
        + and_join([rf"$({b(p['s'], p['i'])}, {b(p['s'], p['n'])})$ in degree ${p['t']}$" for p in pruned])
        + "."
    )
else:
    hl.append(r"The Gebauer--M\"oller divisibility criterion prunes no pair in this range.")
if tot["chain"] == 0:
    hl.append("The chain criterion never applies.")
if boundaries:
    hl.append(
        "A cycle turns out to be a boundary of another cycle of its degree at "
        + and_join([f"level ${s}$ in degree ${t}$" for t, s in boundaries])
        + "."
    )
else:
    hl.append("No cycle is a boundary of another cycle of its degree.")
if args.summary:
    steps = ""
    result = r"\paragraph{Pruning and boundaries.} " + " ".join(hl) + "\n\n"
    lead = "We record"
else:
    steps = "\\section{Step by step}\n\n" + r"\paragraph{Where to look.} " + " ".join(hl) + "\n\n" + "\n".join(body) + "\n"
    result = ""
    lead = "We follow, one reduction at a time,"

result += "\n".join(res) + "\n\n" + "\n".join(GB) + "\n\n" + "\n".join(ST) + "\n"
done = tot["reduced"] - tot["useless"] - tot["beyond"] - tot["boundary"]
result += (
    r"Here ``formed'' counts critical pairs, ``divisible'' and ``chain'' those removed by the "
    rf"Gebauer--M\"oller criteria, and ``steps'' the reduction steps. Of the {tot['reduced']} "
    rf"reductions, {tot['useless']} were useless, {tot['boundary']} found a boundary and "
    rf"{tot['beyond']} went beyond the range; the other {done} each produced a basis element or a "
    "generator.\n"
)

positions = []
for j in range(1, 10):
    for i in range(j - 1, -1, -1):
        deg = 2**i * (2 ** (j - i) - 1) if P == 2 else 2 * P**i * (P ** (j - i) - 1)
        if deg <= S_MAX + N_MAX:
            positions.append((deg, rf"$x_{{{i}{j - i}}}$ ({deg})"))
positions = sorted(positions)
indec = sorted({NAMES[s][j] for s in range(1, len(gens)) for j in range(len(gens[s]))
                if NAMES[s][j] in INDEC.values() and not NAMES[s][j].startswith("h_")})
note = (
    "The remaining generators are named after the indecomposable classes "
    + ", ".join(f"${x}$" for x in indec)
    + " of the standard Adams chart."
    if indec
    else ""
)
if P != 2:
    note += (
        r" The other generators that are not such products are labelled $x_{n,s}$ by their stem "
        r"$n$ and homological degree $s$, with primes to tell apart several in one bidegree."
    )
tex = open(args.template).read()
for key, val in {
    "S_MAX": S_MAX,
    "N_MAX": N_MAX,
    "T_MAX": S_MAX + N_MAX,
    "N_MAX1": N_MAX + 1,
    "N_MAX2": N_MAX + 2,
    "POSITIONS": ", ".join(p for _, p in sorted(positions)),
    "NAME_NOTE": note,
    "STEPS": steps,
    "ABSTRACT_LEAD": lead,
    "RESULT": result + "\n\\end{document}",
}.items():
    tex = tex.replace(f"@{key}@", str(val))
assert "@" not in tex.replace(r"\@", "")
open(args.out, "w").write(tex)
