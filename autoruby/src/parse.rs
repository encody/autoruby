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
pub struct TextEntry<'a> {
    pub text: &'a str,
    pub reading: &'a str,
    pub reading_spans: Vec<ReadingSpan<'a>>,
}

#[derive(Debug)]
pub struct ReadingSpan<'a> {
    pub start_index: u8,
    pub end_index: u8,
    pub text: &'a str,
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

pub fn take_reading_span(input: &str) -> IResult<&str, ReadingSpan> {
    map(
        separated_pair(take_range, char(':'), take_till1(|c| c == '\n' || c == ';')),
        |((start_index, end_index), text)| ReadingSpan {
            start_index,
            end_index,
            text,
        },
    )(input)
}

pub fn take_reading_spans(input: &str) -> IResult<&str, Vec<ReadingSpan>> {
    separated_list0(char(';'), take_reading_span)(input)
}

pub fn dictionary_line(input: &str) -> IResult<&str, TextEntry> {
    map(
        tuple((
            take_until("|"),
            char('|'),
            take_until("|"),
            char('|'),
            take_reading_spans,
        )),
        |(text, _, reading, _, reading_spans)| TextEntry {
            text,
            reading,
            reading_spans,
        },
    )(input)
}
