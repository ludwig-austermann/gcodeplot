extern crate pest;

use pest::{ Parser, error::Error, iterators::Pair };

/// handles all (limited) gcode
#[allow(non_snake_case)]
pub enum GCodeExpr<'a> {
    Home,
    Move { X: f32, Y: f32 },
    Arc { CLKW: bool, X: f32, Y: f32, I: f32, J: f32 },
    Pen(bool), // true => PENDOWN
    Comment(&'a str),
}

/// without Comments, for faster and more memory efficient usecases
#[allow(non_snake_case)]
pub enum CommentlessGCodeExpr {
    Home,
    Move { X: f32, Y: f32 },
    Arc { CLKW: bool, X: f32, Y: f32, I: f32, J: f32 },
    Pen(bool), // true => PENDOWN
}

impl GCodeExpr<'_> {
    pub fn as_str(&self) -> String {
        match self {
            GCodeExpr::Home => "G28".to_string(),
            GCodeExpr::Move{X: x, Y: y} => format!("G1 X{} Y{}", x, y),
            GCodeExpr::Arc{CLKW: clkw, X: x, Y: y, I: i, J: j} => if *clkw {
                format!("G2 X{} Y{} I{} J{}", x, y, i, j)
            } else {
                format!("G3 X{} Y{} I{} J{}", x, y, i, j)
            },
            GCodeExpr::Pen(down) => if *down { "M280 P0 S50".to_string() } else { "M280 P0 S0".to_string() },
            GCodeExpr::Comment(s) => format!(";{}", s),
        }
    }

    pub fn to_commentless(&self) -> CommentlessGCodeExpr {
        match self {
            GCodeExpr::Home => CommentlessGCodeExpr::Home,
            GCodeExpr::Move{X: x, Y: y} => CommentlessGCodeExpr::Move{X: *x, Y: *y},
            GCodeExpr::Arc{CLKW: clkw, X: x, Y: y, I: i, J: j} => CommentlessGCodeExpr::Arc{CLKW: *clkw, X: *x, Y: *y, I: *i, J: *j},
            GCodeExpr::Pen(down) => CommentlessGCodeExpr::Pen(*down),
            GCodeExpr::Comment(_) => panic!("Tried to convert Comment to commentless..."),
        }
    }
}

#[derive(Parser)]
#[grammar = "gcode.pest"]
struct GCodeParser;

pub fn parse_gcode_file(file: &str) -> Result<Vec<(usize, GCodeExpr)>, Error<Rule>> {
    let mut commands = Vec::new();
    let gcode = GCodeParser::parse(Rule::file, file)?;
    for (l, pair) in gcode.enumerate() {
        match pair.as_rule() {
            Rule::expr => {
                for expr in pair.into_inner() {
                    commands.push( (l, parse_expr( expr )) );
                }
            },
            Rule::EOI | Rule::COMMENT => {}, // end of input and empty line
            _ => unreachable!(),
        }
    }
    Ok(commands)
}

fn parse_expr(pair: Pair<Rule>) -> GCodeExpr {
    match pair.as_rule() {
        Rule::HOME => {
            GCodeExpr::Home
        },
        Rule::MOVE => {
            let mut values = (0f32, 0f32);
            for var in pair.into_inner() {
                match var.as_rule() {
                    Rule::X => {
                        values.0 = var.into_inner().as_str().parse::<f32>().unwrap();
                    },
                    Rule::Y => {
                        values.1 = var.into_inner().as_str().parse::<f32>().unwrap();
                    },
                    _ => unreachable!(),
                }
            }
            GCodeExpr::Move { X: values.0, Y: values.1 }
        },
        Rule::ARC  => {
            let mut values = (false, 0f32, 0f32, 0f32, 0f32);
            for var in pair.into_inner() {
                match var.as_rule() {
                    Rule::CLKW => { values.0 = true }
                    Rule::ANTICLKW => { values.0 = false }
                    Rule::X => {
                        values.1 = var.into_inner().as_str().parse::<f32>().unwrap();
                    },
                    Rule::Y => {
                        values.2 = var.into_inner().as_str().parse::<f32>().unwrap();
                    },
                    Rule::I => {
                        values.3 = var.into_inner().as_str().parse::<f32>().unwrap();
                    },
                    Rule::J => {
                        values.4 = var.into_inner().as_str().parse::<f32>().unwrap();
                    },
                    _ => unreachable!(),
                }
            }
            GCodeExpr::Arc {CLKW: values.0, X: values.1, Y: values.2, I: values.3, J: values.4 }
        },
        Rule::PEN  => {
            GCodeExpr::Pen(
                pair.into_inner().as_str().parse::<f32>().unwrap() >= 40.0
            )
        },
        Rule::COMMENT => {
            GCodeExpr::Comment(pair.into_inner().as_str())
        },
        _ => unreachable!(),
    }
}

/// saves commands to gcode
pub fn save(filename: &str, commands: Vec<(usize, GCodeExpr)>) {
    let mut last_l = 0;
    std::fs::write(
        filename,
        commands.iter().map(|(l, cmd)| (l, cmd.as_str())).fold("".to_string(), |mut res, (l, s)| {
            if last_l != *l {
                res.push('\n')
            };
            last_l = *l;
            res.push_str(&s);
            res
        })
        //collect::<Vec<String>>().join("\n")
    ).expect("Unable to save the gcode.");
}