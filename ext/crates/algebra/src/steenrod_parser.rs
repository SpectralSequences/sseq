use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric0, char},
    character::complete::{digit1 as digit, space0 as space},
    combinator::{map_res, opt},
    error::ErrorKind::Char,
    multi::fold_many0,
    multi::many0,
    sequence::{delimited, pair, tuple},
    IResult,
};

use crate::algebra::milnor_algebra::PPart;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum AlgebraBasisElt {
    AList(Vec<BocksteinOrSq>), // Admissible list.
    PList(PPart),
    P(u32),
    Q(u32),
}

#[derive(Debug, Clone)]
pub enum AlgebraParseNode {
    Product(Box<AlgebraParseNode>, Box<AlgebraParseNode>),
    Sum(Box<AlgebraParseNode>, Box<AlgebraParseNode>),
    BasisElt(AlgebraBasisElt),
    Scalar(i32),
}

#[derive(Debug, Clone)]
pub enum ModuleParseNode {
    Act(Box<AlgebraParseNode>, Box<ModuleParseNode>),
    Sum(Box<ModuleParseNode>, Box<ModuleParseNode>),
    ModuleBasisElt(String),
}

fn digits<T: FromStr + Copy>(i: &str) -> IResult<&str, T> {
    map_res(delimited(space, digit, space), FromStr::from_str)(i)
}

fn comma_separated_integer_list<T: FromStr + Copy>(i: &str) -> IResult<&str, Vec<T>> {
    let (i, init) = digits(i)?;
    let mut result = vec![init];
    let (rest, list) = many0(pair(char(','), digits))(i)?;
    result.extend(list.iter().map(|t: &(char, T)| t.1));
    Ok((rest, result))
}

fn comma_separated_sequence<T: FromStr + Copy>(i: &str) -> IResult<&str, Vec<T>> {
    delimited(tag("("), comma_separated_integer_list, tag(")"))(i)
}

fn space_separated_integer_list<T: FromStr + Copy>(i: &str) -> IResult<&str, Vec<T>> {
    many0(digits)(i)
}

fn space_separated_sequence<T: FromStr + Copy>(i: &str) -> IResult<&str, Vec<T>> {
    delimited(tag("("), space_separated_integer_list, tag(")"))(i)
}

#[derive(Clone, Debug)]
pub enum BocksteinOrSq {
    Bockstein,
    Sq(u32),
}

fn bockstein_b(i: &str) -> IResult<&str, BocksteinOrSq> {
    let (rest, _c) = char('b')(i)?;
    Ok((rest, BocksteinOrSq::Bockstein))
}

fn sq_digits(i: &str) -> IResult<&str, BocksteinOrSq> {
    let (rest, c) = digits(i)?;
    Ok((rest, BocksteinOrSq::Sq(c)))
}

fn space_separated_bockstein_or_sq_list(i: &str) -> IResult<&str, Vec<BocksteinOrSq>> {
    many0(alt((bockstein_b, sq_digits)))(i)
}

fn space_separated_bockstein_or_sq_sequence(i: &str) -> IResult<&str, Vec<BocksteinOrSq>> {
    delimited(tag("("), space_separated_bockstein_or_sq_list, tag(")"))(i)
}

fn algebra_generator(i: &str) -> IResult<&str, AlgebraParseNode> {
    let (rest, opt_elt) = opt(alt((
        pair(tag("Q"), digits),
        pair(tag("P"), digits),
        pair(tag("Sq"), digits),
    )))(i)?;

    if let Some(elt) = opt_elt {
        let result = match elt {
            ("Q", x) => AlgebraBasisElt::Q(x),
            ("P", x) | ("Sq", x) => AlgebraBasisElt::P(x),
            _ => unreachable!(),
        };
        return Ok((rest, AlgebraParseNode::BasisElt(result)));
    }
    if let Ok((rest, elt)) = alt((
        pair(tag("P"), comma_separated_sequence),
        pair(tag("Sq"), comma_separated_sequence),
        pair(tag("M"), space_separated_sequence),
    ))(i)
    {
        let result = match elt {
            ("P", x) | ("Sq", x) | ("M", x) => AlgebraBasisElt::PList(x),
            _ => unreachable!(),
        };
        return Ok((rest, AlgebraParseNode::BasisElt(result)));
    }
    let (rest, elt) = pair(tag("A"), space_separated_bockstein_or_sq_sequence)(i)?;
    {
        let result = match elt {
            ("A", x) => AlgebraBasisElt::AList(x),
            _ => unreachable!(),
        };
        Ok((rest, AlgebraParseNode::BasisElt(result)))
    }
}

fn scalar(i: &str) -> IResult<&str, AlgebraParseNode> {
    let (rest, c) = pair(opt(alt((char('-'), char('+')))), digits)(i)?;
    let result: i32 = match c {
        (Some('+'), coeff) | (None, coeff) => coeff,
        (Some('-'), coeff) => -coeff,
        _ => unreachable!(),
    };
    Ok((rest, AlgebraParseNode::Scalar(result)))
}

fn algebra_parens(i: &str) -> IResult<&str, AlgebraParseNode> {
    delimited(space, delimited(tag("("), algebra_expr, tag(")")), space)(i)
}

fn algebra_factor(i: &str) -> IResult<&str, AlgebraParseNode> {
    alt((
        delimited(space, algebra_generator, space),
        scalar,
        algebra_parens,
    ))(i)
}

// We read an initial factor and for each time we find
// a * or / operator followed by another factor, we do
// the math by folding everything
fn algebra_term(i: &str) -> IResult<&str, AlgebraParseNode> {
    let (i, init) = pair(opt(alt((char('+'), char('-')))), algebra_factor)(i)?;
    let first_factor = || match &init {
        (Some('+'), fact) | (None, fact) => fact.clone(),
        (Some('-'), fact) => AlgebraParseNode::Product(
            Box::new(AlgebraParseNode::Scalar(-1)),
            Box::new(fact.clone()),
        ),
        _ => unreachable!(),
    };
    // This is necessary for lifetime reasons.
    #[allow(clippy::let_and_return)]
    let result = fold_many0(
        pair(alt((char('*'), char(' '))), algebra_factor),
        first_factor,
        |acc, (_op, val): (char, AlgebraParseNode)| {
            AlgebraParseNode::Product(Box::new(acc), Box::new(val))
        },
    )(i);
    result
}

fn algebra_expr(i: &str) -> IResult<&str, AlgebraParseNode> {
    let (i, init) = algebra_term(i)?;

    // This is necessary for lifetime reasons.
    #[allow(clippy::let_and_return)]
    let result = fold_many0(
        pair(alt((char('+'), char('-'))), algebra_term),
        || init.clone(),
        |acc, (_op, val): (char, AlgebraParseNode)| {
            AlgebraParseNode::Sum(Box::new(acc), Box::new(val))
        },
    )(i);
    result
}

fn module_generator(i: &str) -> IResult<&str, ModuleParseNode> {
    let (rest, (a, more_str)) = pair(alpha1, alphanumeric0)(i)?;
    if a.starts_with("Sq") || a.starts_with('P') || a.starts_with('Q') {
        return Err(nom::Err::Error(nom::error::Error::new(
            "Module generators are not allowed to start with P, Q, or Sq",
            Char,
        )));
    }
    Ok((
        rest,
        ModuleParseNode::ModuleBasisElt(a.to_string() + more_str),
    ))
}

fn module_parens(i: &str) -> IResult<&str, ModuleParseNode> {
    delimited(space, delimited(tag("("), module_expr, tag(")")), space)(i)
}

fn module_factor(i: &str) -> IResult<&str, ModuleParseNode> {
    alt((delimited(space, module_generator, space), module_parens))(i)
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
        result = ModuleParseNode::Act(Box::new(AlgebraParseNode::Scalar(-1)), Box::new(result));
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

pub fn parse_algebra(i: &str) -> Result<AlgebraParseNode, ParseError> {
    let (rest, parse_tree) = algebra_expr(i).map_err(|err| ParseError {
        info: format!("{:#?}", err),
    })?;
    if rest.is_empty() {
        Ok(parse_tree)
    } else {
        Err(ParseError {
            info: "Failed to consume all of input".to_string(),
        })
    }
}

pub fn parse_module(i: &str) -> Result<ModuleParseNode, ParseError> {
    let (rest, parse_tree) = module_expr(i).map_err(|err| ParseError {
        info: format!("{:#?}", err),
    })?;
    if rest.is_empty() {
        Ok(parse_tree)
    } else {
        Err(ParseError {
            info: "Failed to consume all of input".to_string(),
        })
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub info: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error:\n    {}\n", &self.info)
    }
}

impl std::error::Error for ParseError {
    fn description(&self) -> &str {
        "Parse error"
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
