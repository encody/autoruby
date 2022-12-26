use std::num::ParseIntError;

use nom::{
    bytes::complete::{take_till1, take_until},
    character::complete::{char, digit1},
    combinator::{map, map_res, opt},
    multi::separated_list0,
    sequence::{preceded, separated_pair, tuple},
    IResult,
};

#[derive(Debug)]
pub struct FuriganaEntry<'a> {
    pub text: &'a str,
    pub reading: &'a str,
    pub rubies: Vec<RubyEntry<'a>>,
}

#[derive(Debug)]
pub struct RubyEntry<'a> {
    pub start_index: u8,
    pub end_index: u8,
    pub rt: &'a str,
}

pub fn take_range(input: &str) -> IResult<&str, (u8, u8)> {
    map_res(
        tuple((digit1, opt(preceded(char('-'), digit1)))),
        |(start, end): (&str, Option<&str>)| {
            let start: u8 = start.parse()?;
            let end = if let Some(end) = end {
                end.parse()?
            } else {
                start
            };
            Ok::<_, ParseIntError>((start, end))
        },
    )(input)
}

pub fn take_ruby(input: &str) -> IResult<&str, RubyEntry> {
    map(
        separated_pair(take_range, char(':'), take_till1(|c| c == '\n' || c == ';')),
        |((start_index, end_index), rt)| RubyEntry {
            start_index,
            end_index,
            rt,
        },
    )(input)
}

pub fn take_rubies(input: &str) -> IResult<&str, Vec<RubyEntry>> {
    separated_list0(char(';'), take_ruby)(input)
}

pub fn dictionary_line(input: &str) -> IResult<&str, FuriganaEntry> {
    map(
        tuple((
            take_until("|"),
            char('|'),
            take_until("|"),
            char('|'),
            take_rubies,
        )),
        |(text, _, reading, _, rubies)| FuriganaEntry {
            text,
            reading,
            rubies,
        },
    )(input)
}
