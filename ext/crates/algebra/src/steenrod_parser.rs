use anyhow::{anyhow, Context};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric0, char},
    character::complete::{digit1 as digit, space0},
    combinator::{map, map_res, opt, peek},
    error::{ParseError, VerboseError, VerboseErrorKind},
    multi::many0,
    multi::{fold_many0, separated_list1},
    sequence::{delimited, pair, preceded, tuple},
    IResult as IResultBase, Parser,
};

use crate::algebra::milnor_algebra::PPart;
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

#[derive(Debug, Clone)]
pub enum ModuleParseNode {
    Act(Box<AlgebraNode>, Box<ModuleParseNode>),
    Sum(Box<ModuleParseNode>, Box<ModuleParseNode>),
    ModuleBasisElt(String),
}

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

#[derive(Clone, Debug)]
pub enum BocksteinOrSq {
    Bockstein,
    Sq(u32),
}

fn algebra_generator(i: &str) -> IResult<&str, AlgebraBasisElt> {
    alt((
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

fn module_generator(i: &str) -> IResult<&str, ModuleParseNode> {
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
    Ok((
        rest,
        ModuleParseNode::ModuleBasisElt(a.to_string() + more_str),
    ))
}

fn module_factor(i: &str) -> IResult<&str, ModuleParseNode> {
    space(alt((module_generator, brackets(module_expr))))(i)
}

fn module_term(i: &str) -> IResult<&str, ModuleParseNode> {
    // println!("hi");
    let (rest, (opt_pm, opt_algebra, mut result)) = tuple((
        opt(alt((char('+'), char('-')))),
        opt(pair(algebra_expr, char('*'))),
        module_factor,
    ))(i)?;
    // println!("{:?}, {:?}, {:?}", opt_pm, opt_algebra, result);
    if let Some((algebra_term, _)) = opt_algebra {
        result = ModuleParseNode::Act(Box::new(algebra_term), Box::new(result));
    }
    if let Some('-') = opt_pm {
        result = ModuleParseNode::Act(Box::new(AlgebraNode::Scalar(-1)), Box::new(result));
    }
    Ok((rest, result))
}

fn module_expr(i: &str) -> IResult<&str, ModuleParseNode> {
    let (i, init) = module_term(i)?;
    // This is necessary for lifetime reasons.
    #[allow(clippy::let_and_return)]
    let result = fold_many0(
        pair(alt((char('+'), char('-'))), module_term),
        || init.clone(),
        |acc, (_op, val): (char, ModuleParseNode)| {
            ModuleParseNode::Sum(Box::new(acc), Box::new(val))
        },
    )(i);
    result
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

pub fn parse_module(i: &str) -> anyhow::Result<ModuleParseNode> {
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

    #[test]
    fn test_parser() {
        println!();

        println!("{:?}", parse_algebra("Sq(1,2)+Sq2 + A(2 b 2 3)").unwrap());

        println!();
    }
}
