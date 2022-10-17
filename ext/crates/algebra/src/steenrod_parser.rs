//! This module includes code for parsing an expression in the Steenrod algebra into an abstract
//! syntax tree.

use anyhow::{anyhow, Context};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric0, char},
    character::complete::{digit1 as digit, space0},
    combinator::{map, map_res, opt, peek},
    error::{context, ParseError, VerboseError, VerboseErrorKind},
    multi::{many0, separated_list1},
    sequence::{delimited, pair, preceded},
    IResult as IResultBase, Parser,
};

use crate::{adem_algebra::AdemBasisElement, algebra::milnor_algebra::PPart};
use std::str::FromStr;

type IResult<I, O> = IResultBase<I, O, VerboseError<I>>;

#[derive(Debug, Clone)]
pub enum AlgebraBasisElt {
    AList(Vec<BocksteinOrSq>), // Admissible list.
    PList(PPart),
    P(u32),
    Q(u32),
}

#[derive(Debug, Clone)]
pub enum AlgebraNode {
    Product(Box<AlgebraNode>, Box<AlgebraNode>),
    Sum(Box<AlgebraNode>, Box<AlgebraNode>),
    BasisElt(AlgebraBasisElt),
    Scalar(i32),
}

pub type ModuleNode = Vec<(AlgebraNode, String)>;

/// Pad both ends with whitespace
pub(crate) fn space<'a, O, E: ParseError<&'a str>, F: Parser<&'a str, O, E>>(
    f: F,
) -> impl FnMut(&'a str) -> IResultBase<&'a str, O, E> {
    delimited(space0, f, space0)
}

/// Surround with brackets
pub(crate) fn brackets<'a, O, E: ParseError<&'a str>, F: Parser<&'a str, O, E>>(
    f: F,
) -> impl FnMut(&'a str) -> IResultBase<&'a str, O, E> {
    delimited(char('('), f, char(')'))
}

pub(crate) fn digits<T: FromStr + Copy>(i: &str) -> IResult<&str, T> {
    map_res(space(digit), FromStr::from_str)(i)
}

pub(crate) fn p_or_sq(i: &str) -> IResult<&str, &str> {
    alt((tag("P"), tag("Sq")))(i)
}

fn fold_separated<I: Clone, OS, O, E>(
    mut sep: impl Parser<I, OS, E>,
    mut f: impl Parser<I, O, E>,
    mut acc: impl FnMut(O, O) -> O,
) -> impl FnMut(I) -> IResultBase<I, O, E> {
    move |i: I| {
        let (mut i, mut res) = f.parse(i)?;
        loop {
            match sep.parse(i.clone()) {
                Err(nom::Err::Error(_)) => return Ok((i, res)),
                Err(e) => return Err(e),
                Ok((i1, _)) => match f.parse(i1.clone()) {
                    Err(nom::Err::Error(_)) => return Ok((i, res)),
                    Err(e) => return Err(e),
                    Ok((i2, o)) => {
                        i = i2;
                        res = acc(res, o);
                    }
                },
            }
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum BocksteinOrSq {
    Bockstein,
    Sq(u32),
}

impl BocksteinOrSq {
    pub(crate) fn to_adem_basis_elt(self, q: i32) -> AdemBasisElement {
        match self {
            BocksteinOrSq::Bockstein => {
                if q == 1 {
                    AdemBasisElement {
                        degree: 1,
                        bocksteins: 0,
                        ps: vec![1],
                        p_or_sq: false,
                    }
                } else {
                    AdemBasisElement {
                        degree: 1,
                        bocksteins: 1,
                        ps: vec![],
                        p_or_sq: true,
                    }
                }
            }
            BocksteinOrSq::Sq(x) => AdemBasisElement {
                degree: x as i32 * q,
                bocksteins: 0,
                ps: vec![x],
                p_or_sq: q != 1,
            },
        }
    }
}

fn algebra_generator(i: &str) -> IResult<&str, AlgebraBasisElt> {
    alt((
        map(char('b'), |_| AlgebraBasisElt::Q(0)),
        map(preceded(char('Q'), digits), AlgebraBasisElt::Q),
        map(preceded(p_or_sq, digits), AlgebraBasisElt::P),
        map(
            alt((
                preceded(p_or_sq, brackets(separated_list1(char(','), digits))),
                preceded(char('M'), brackets(many0(digits))),
            )),
            AlgebraBasisElt::PList,
        ),
        map(
            preceded(
                char('A'),
                brackets(many0(alt((
                    map(char('b'), |_| BocksteinOrSq::Bockstein),
                    map(digits, BocksteinOrSq::Sq),
                )))),
            ),
            AlgebraBasisElt::AList,
        ),
    ))(i)
}

fn scalar(i: &str) -> IResult<&str, i32> {
    alt((
        digits,
        preceded(char('+'), digits),
        map(preceded(char('-'), digits), |x: i32| -x),
    ))(i)
}

fn algebra_factor(i: &str) -> IResult<&str, AlgebraNode> {
    space(alt((
        map(algebra_generator, AlgebraNode::BasisElt),
        map(scalar, AlgebraNode::Scalar),
        brackets(algebra_expr),
    )))(i)
}

fn algebra_term(i: &str) -> IResult<&str, AlgebraNode> {
    let (i, sign) = opt(alt((char('+'), char('-'))))(i)?;

    let (i, mut res) = fold_separated(char('*'), algebra_factor, |acc, val| {
        AlgebraNode::Product(Box::new(acc), Box::new(val))
    })(i)?;

    if let Some('-') = sign {
        res = AlgebraNode::Product(Box::new(AlgebraNode::Scalar(-1)), Box::new(res));
    }
    Ok((i, res))
}

fn algebra_expr(i: &str) -> IResult<&str, AlgebraNode> {
    fold_separated(
        peek(alt((char('+'), char('-')))),
        space(algebra_term),
        |acc, val| AlgebraNode::Sum(Box::new(acc), Box::new(val)),
    )(i)
}

fn module_generator(i: &str) -> IResult<&str, String> {
    let (rest, (a, more_str)) = pair(alpha1, alphanumeric0)(i)?;
    if a.starts_with("Sq") || a.starts_with('P') || a.starts_with('Q') {
        return Err(nom::Err::Failure(VerboseError {
            errors: vec![(
                &i[0..a.len()],
                VerboseErrorKind::Context(
                    "Module generators are not allowed to start with P, Q, or Sq",
                ),
            )],
        }));
    }
    Ok((rest, a.to_string() + more_str))
}

fn module_term(i: &str) -> IResult<&str, ModuleNode> {
    use AlgebraNode::*;

    let (i, prefix) = opt(alt((
        map(pair(space(algebra_term), char('*')), |(a, _)| a),
        map(char('-'), |_| Scalar(-1)),
        map(char('+'), |_| Scalar(1)),
    )))(i)
    .unwrap();

    match space(module_generator)(i) {
        #[allow(clippy::or_fun_call)] // Otherwise it triggers clippy::unnecessary_lazy_evaluations
        Ok((i, gen)) => return Ok((i, vec![(prefix.unwrap_or(Scalar(1)), gen)])),
        Err(nom::Err::Error(_)) => (),
        Err(e) => return Err(e),
    }

    let (i, expr) = context("Parsing bracketed expression", space(brackets(module_expr)))(i)?;
    Ok((
        i,
        match prefix {
            Some(a) => expr
                .into_iter()
                .map(|(b, v)| (Product(Box::new(a.clone()), Box::new(b)), v))
                .collect(),
            None => expr,
        },
    ))
}

fn module_expr(i: &str) -> IResult<&str, ModuleNode> {
    fold_separated(
        peek(alt((char('+'), char('-')))),
        module_term,
        |mut a, b| {
            a.extend_from_slice(&b);
            a
        },
    )(i)
}

fn convert_error(i: &str) -> impl FnOnce(nom::Err<VerboseError<&str>>) -> anyhow::Error + '_ {
    move |err| {
        anyhow!(match err {
            nom::Err::Error(e) | nom::Err::Failure(e) => nom::error::convert_error(i, e),
            _ => format!("{err:#}"),
        })
    }
}

pub fn parse_algebra(i: &str) -> anyhow::Result<AlgebraNode> {
    let (rest, parse_tree) = algebra_expr(i)
        .map_err(convert_error(i))
        .with_context(|| format!("Error when parsing algebra string {i}"))?;
    if rest.is_empty() {
        Ok(parse_tree)
    } else {
        Err(anyhow!(
            "Failed to consume all of input. Remaining: '{rest}'"
        ))
    }
}

pub fn parse_module(i: &str) -> anyhow::Result<ModuleNode> {
    let (rest, parse_tree) = module_expr(i)
        .map_err(convert_error(i))
        .with_context(|| format!("Error when parsing module string {i}"))?;
    if rest.is_empty() {
        Ok(parse_tree)
    } else {
        Err(anyhow!(
            "Failed to consume all of input. Remaining: '{rest}'"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};

    #[test]
    fn test_parse_algebra() {
        let check = |input, output: Expect| {
            output.assert_eq(&format!("{:?}", parse_algebra(input).unwrap()));
        };

        check(
            "b * Q3 * (Sq1 * A(2 b 5) + M(0 0 2) * P(0, 1) * Sq(1, 0))",
            expect![[
                r#"Product(Product(BasisElt(Q(0)), BasisElt(Q(3))), Sum(Product(BasisElt(P(1)), BasisElt(AList([Sq(2), Bockstein, Sq(5)]))), Product(Product(BasisElt(PList([0, 0, 2])), BasisElt(PList([0, 1]))), BasisElt(PList([1, 0])))))"#
            ]],
        );
    }

    #[test]
    fn test_parse_module() {
        let check = |input, output: Expect| {
            output.assert_eq(&format!("{:?}", parse_module(input).unwrap()));
        };

        check("x0", expect![[r#"[(Scalar(1), "x0")]"#]]);

        check("Sq2 * x0", expect![[r#"[(BasisElt(P(2)), "x0")]"#]]);

        check(
            "Sq1 * x0 + x1",
            expect![[r#"[(BasisElt(P(1)), "x0"), (Scalar(1), "x1")]"#]],
        );

        check(
            "(Sq3 + Sq2 * Sq1) * x0",
            expect![[r#"[(Sum(BasisElt(P(3)), Product(BasisElt(P(2)), BasisElt(P(1)))), "x0")]"#]],
        );

        check(
            "Sq3 * x0 + Sq2 * x1",
            expect![[r#"[(BasisElt(P(3)), "x0"), (BasisElt(P(2)), "x1")]"#]],
        );

        check(
            "(Sq3 - Sq2 * Sq1) * (Sq1 * x0 + x1)",
            expect![[
                r#"[(Product(Sum(BasisElt(P(3)), Product(Scalar(-1), Product(BasisElt(P(2)), BasisElt(P(1))))), BasisElt(P(1))), "x0"), (Product(Sum(BasisElt(P(3)), Product(Scalar(-1), Product(BasisElt(P(2)), BasisElt(P(1))))), Scalar(1)), "x1")]"#
            ]],
        );

        check(
            "x3 - (Sq3 * x0 + Sq1 * x2)",
            expect![[
                r#"[(Scalar(1), "x3"), (Product(Scalar(-1), BasisElt(P(3))), "x0"), (Product(Scalar(-1), BasisElt(P(1))), "x2")]"#
            ]],
        );
    }

    #[test]
    fn test_parse_module_errors() {
        // Checking the error output breaks because it depends on whether backtraces are available.
        let check = |input| {
            assert!(parse_module(input).is_err());
        };

        check("x0 + ");
        check("2 * (x1 + Sq1 * x0");
        check("Sqx");
    }
}
