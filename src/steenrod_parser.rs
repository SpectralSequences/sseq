use nom::{
  IResult,
  branch::alt,
  bytes::complete::tag,  
  combinator::{map_res, opt},
  character::complete::{char, alphanumeric0, alpha1},
  character::complete::{digit1 as digit, space0 as space},
  error::ErrorKind::Char,
  multi::fold_many0,
  multi::many0,
  sequence::{delimited, pair, tuple}
};

use std::error::Error;
use std::str::FromStr;

#[derive(Debug)]
#[derive(Clone)]
pub enum AlgebraBasisElt {
    PList(Vec<u32>),
    P(u32),
    Q(u32)
}

#[derive(Debug)]
#[derive(Clone)]
pub enum AlgebraParseNode {
    Product(Box<AlgebraParseNode>, Box<AlgebraParseNode>),
    Sum(Box<AlgebraParseNode>, Box<AlgebraParseNode>),
    BasisElt(AlgebraBasisElt),
    Scalar(i32)
}

#[derive(Debug)]
#[derive(Clone)]
pub enum ModuleParseNode {
    Act(Box<AlgebraParseNode>, Box<ModuleParseNode>),
    Sum(Box<ModuleParseNode>, Box<ModuleParseNode>),
    ModuleBasisElt(String)
}


fn digits(i : &str) -> IResult<&str, u32> {
    map_res(delimited(space, digit, space), FromStr::from_str)(i)
}

fn integer_list(i : &str) -> IResult<&str, Vec<u32>> {
    let (i, init) = digits(i)?;
    let mut result = vec![init];
    let (rest, list) = many0(pair(char(','), digits))(i)?;
    result.extend(list.iter().map(|t| t.1));
    return Ok((rest, result));
}

fn sequence(i : &str) -> IResult<&str, Vec<u32>> {
    delimited(
      tag("("),
      integer_list,
      tag(")")
    )(i)
}

fn algebra_generator(i : &str) -> IResult<&str, AlgebraParseNode> {
    let (mut rest, opt_elt) = opt(alt((
        pair(tag("Q"), digits),
        pair(tag("P"), digits),
        pair(tag("Sq"), digits)
    )))(i)?;
    
    let result;    
    if let Some(elt) = opt_elt {
        match elt {
            ("Q", x ) => result = AlgebraBasisElt::Q(x),
            ("P", x ) | ( "Sq", x) => result = AlgebraBasisElt::P(x),
            _ => unreachable!()
        };
    } else {
        let (new_rest, elt) = alt((
            pair(tag("P"), sequence),
            pair(tag("Sq"), sequence)
        ))(i)?;
        rest = new_rest;
        match elt {
            ("P", x ) | ( "Sq", x) => result = AlgebraBasisElt::PList(x),
            _ => unreachable!()
        }
    }
    // ("P", x ) | ( "Sq", x) => BasisElt::PList(x),
    return Ok((rest, AlgebraParseNode::BasisElt(result)));
}

fn scalar(i : &str) -> IResult<&str, AlgebraParseNode> {
    let (rest, c) =  pair(opt(alt((char('-'),char('+')))), digits)(i)?;
    let result = match c {
        (Some('+'), coeff) | (None, coeff) => coeff as i32,
        (Some('-'), coeff) => -(coeff as i32),
        _ => unreachable!()
    };
    Ok((rest,AlgebraParseNode::Scalar(result)))
}

fn algebra_parens(i: &str) -> IResult<&str, AlgebraParseNode> {
  delimited(
    space,
    delimited(
      tag("("),
      algebra_expr,
      tag(")")
    ),
    space
  )(i)
}

fn algebra_factor(i: &str) -> IResult<&str, AlgebraParseNode> {
  alt((
    delimited(space, algebra_generator, space),
    scalar,
    algebra_parens
  ))(i)
}

// We read an initial factor and for each time we find
// a * or / operator followed by another factor, we do
// the math by folding everything
fn algebra_term(i: &str) -> IResult<&str, AlgebraParseNode> {
    let (i, init) = pair(opt(alt((char('+'), char('-')))), algebra_factor)(i)?;
    let first_factor = match init {
        (Some('+'), fact) | (None, fact) => fact,
        (Some('-'), fact) => AlgebraParseNode::Product(Box::new(AlgebraParseNode::Scalar(-1)), Box::new(fact)),
        _ => unreachable!()
    };  
    fold_many0(
        pair(alt((char('*'), char(' '))), algebra_factor),
        first_factor,
        |acc, (op, val): (char, AlgebraParseNode)| {
            AlgebraParseNode::Product(Box::new(acc), Box::new(val))
        }
    )(i)
}

fn algebra_expr(i: &str) -> IResult<&str, AlgebraParseNode> {
  let (i, init) = algebra_term(i)?;

  fold_many0(
    pair(alt((char('+'), char('-'))), algebra_term),
    init,
    |acc, (op, val): (char, AlgebraParseNode)| {
        AlgebraParseNode::Sum(Box::new(acc), Box::new(val))
    }
  )(i)
}

fn module_generator(i: &str) -> IResult<&str, ModuleParseNode> {
    let (rest, (a, more_str)) = pair(alpha1, alphanumeric0)(i)?;
    if a.starts_with("Sq") || a.starts_with("P") || a.starts_with("Q") {
        return Err(nom::Err::Error(("Module generators are not allowed to start with P, Q, or Sq", Char)));
    }
    Ok((rest, ModuleParseNode::ModuleBasisElt(a.to_string() + more_str)))
}

fn module_parens(i: &str) -> IResult<&str, ModuleParseNode> {
  delimited(
    space,
    delimited(
      tag("("),
      module_expr,
      tag(")")
    ),
    space
  )(i)
}

fn module_factor(i: &str) -> IResult<&str, ModuleParseNode> {
  alt((
    delimited(space, module_generator, space),
    module_parens
  ))(i)
}

fn module_term(i: &str) -> IResult<&str, ModuleParseNode> {
    // println!("hi");
    let (rest, (opt_pm, opt_algebra, mut result)) = tuple((opt(alt((char('+'), char('-')))), opt(pair(algebra_expr, char('*'))), module_factor))(i)?;
    // println!("{:?}, {:?}, {:?}", opt_pm, opt_algebra, result);
    match opt_algebra {
        Some((algebra_term, _)) => result = ModuleParseNode::Act(Box::new(algebra_term), Box::new(result)),
        None => {}
    };
    match opt_pm {
        Some('-') => result = ModuleParseNode::Act(Box::new(AlgebraParseNode::Scalar(-1)), Box::new(result)),
        _ => {}
    };
    Ok((rest,result))
}

fn module_expr(i: &str) -> IResult<&str, ModuleParseNode> {
  let (i, init) = module_term(i)?;
  fold_many0(
    pair(alt((char('+'), char('-'))), module_term),
    init,
    |acc, (op, val): (char, ModuleParseNode)| {
        ModuleParseNode::Sum(Box::new(acc), Box::new(val))
    }
  )(i)
}

pub fn parse_algebra(i : &str) -> Result<AlgebraParseNode, Box<dyn std::error::Error>> {
    let (rest, parse_tree) = algebra_expr(i)
        .or_else(|err| Err(Box::new(ParseError{info : format!("{:#?}", err) })))?;
    if rest.len() > 0 {
        Err(Box::new(ParseError {info : "Failed to consume all of input".to_string()}))
    } else {
        Ok(parse_tree)
    }
}

pub fn parse_module(i : &str) -> Result<ModuleParseNode, Box<dyn std::error::Error>> {
    let (rest, parse_tree) = module_expr(i)
        .or_else(|err| Err(Box::new(ParseError{info : format!("{:#?}", err) })))?;
    if rest.len() > 0 {
        Err(Box::new(ParseError {info : "Failed to consume all of input".to_string()}))
    } else {
        Ok(parse_tree)
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use rstest::rstest_parametrize;

}


#[derive(Debug)]
pub struct ParseError {
    pub info : String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error:\n    {}\n", &self.info)
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        "Parse error"
    }
}
