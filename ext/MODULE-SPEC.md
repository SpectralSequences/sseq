# Module specification

This files explains how one specifies Steenrod modules as json files. Note that
the built-in modules in `steenrod_modules/` often contain unused fields for
historical reasons. There are also multiple module formats, and we are only
describing the simplest version here.

## Examples

### C2 ∧ C2

```json
{
    "p": 2,
    "type": "finite dimensional module",
    "gens": {"x0.x0": 0, "x0.x1": 1, "x1.x0": 1, "x1.x1": 2},
    "actions": [
        "Sq1 x0.x0 = x1.x0 + x0.x1",
        "Sq1 x1.x0 = x1.x1",
        "Sq1 x0.x1 = x1.x1",
        "Sq2 x0.x0 = x1.x1"
    ]
}
```

### C3

```json
{
    "p": 3,
    "type": "finite dimensional module",
    "gens": {"x0": 0, "x1": 1},
    "actions": ["b x0 = x1"]
}
```

### tmf

```json
{
    "p": 2,
    "algebra": ["milnor"],
    "profile": {
        "truncated": true,
        "p_part": [3, 2, 1]
    },
    "type": "finite dimensional module",
    "gens": {"x0": 0},
    "actions": []
}
```

### y(2)

y(2) is a spectrum whose homology is `F_p[ξ₁, ξ₂][τ₁, τ₂]`.

```json
{
    "p": 3,
    "algebra": ["milnor"],
    "profile": {
        "q_part": 4294967292,
        "p_part": [0, 0],
        "truncated": false
    },
    "type" : "finite dimensional module",
    "gens": {"x0": 0},
    "actions": []
}
```

## Algebra

In each of the above examples, the first "block" in the json file specifies the
algebra. Note that the entries can be ordered in any way, and in particular, it
is perfectly acceptable to interleave the algebra and module parts.

### Required parameters

* `p`: The prime we are working at. Currently, we only support primes up to
   19. This cap can be easily lifted if necessary.

### Optional parameters

* `algebra`: There are two standard bases for the Steenrod algebra --- the
   Milnor basis and the Adem basis. The Adem basis is faster than the Milnor
   basis, but only the Milnor basis supports a non-trivial profile (see next
   entry). This parameter specifies which bases the module supports. Usually,
   you will want to set it to `["milnor"]` if you have a non-trivial profile,
   or `["milnor", "adem"]` otherwise.

   The default option is `["milnor", "adem"]`.

   Note that picking a different basis will result in different bases for the
   Ext groups.

* `profile`: This specifies a profile function for the Steenrod algebra. If
   specified, Ext is computed against the specified quotient of the dual
   Steenrod algebra (or rather, the dual of the quotient of the dual Steenrod
   algebra, since we work with algebras instead of coalgebras). There are three
   possible sub-parameters:

  * `truncated`: If `true`, the unspecified `p_part` entries will be 0.
       Otherwise, they will be infinity.
  * `p_part`: The profile function. If this is set to `[r_1, r_2, ...]`,
       then we quotient out by `ξ_i^{p^{r_i}}` for all i in the Steenrod algebra.
  * `q_part`: This is only relevant at odd primes. This is a single integer
       that one should interpret in binary. The nth bit is `0` if we want to
       quotient out by `τ_n`, and `1` otherwise.

## Module

The specification of a module starts with the `type`. The possible values are `finite dimensional module`, `finitely presented module` and `real projective space`.

### Finite Dimensional Module

There are two required parameters

* `gens`: This is a dictionary of generators, specified in the format
   `{"gen_name": gen_degree, ...}`.

* `actions`: This is a list of actions by the generators of the Steenrod
   algebra, which are `Sq^{2^n}` for even primes and `β, P^{p^n}` for odd
   primes. For example, we can have
  * `P3 x0 = 2 x2 + x3`
  * `b x0 = x1`
  * `Sq1 x0 = x1`

   Note that when there is a non-trivial profile, you should not attempt to set
   an action if the generator doesn't exist in the subalgebra.

### Finitely Presented Module

TODO

See [https://github.com/SpectralSequences/ext/blob/master/steenrod_modules/A-mod-Sq1-Sq2.json](https://github.com/SpectralSequences/ext/blob/master/steenrod_modules/A-mod-Sq1-Sq2.json) for an example.

### (Stunted) Real Projective Space

This only works at the prime `2`, resolving `RP_n^m`

* `min`: This is the degree of the lowest dimension cell in the stunted
   projective space. This can be negative.
* `max`: This is the degree of the highest dimension cell in the stunted
   projective space. If unspecified, it is infinity.
* `clear_bottom`: This should only be used if resolving against A(2). If
   selected, this quotients out the elements in the *A(2) submodule* generated
   by degrees less than `min`. This is useful for approximating `tmf ∧ RP_∞^n`,
   c.f. Proposition 2.2 of Bailey and Ricka. Note that this quotient always has
   minimum degree -1 mod 8.

## Products and self maps

TODO

See [https://github.com/SpectralSequences/ext/blob/master/steenrod_modules/C3.json](https://github.com/SpectralSequences/ext/blob/master/steenrod_modules/C3.json) for an example.
