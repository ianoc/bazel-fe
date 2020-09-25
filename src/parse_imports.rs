extern crate nom;

use nom::branch::alt;
use nom::{
    bytes::complete::tag,
    bytes::complete::take_while_m_n,
    combinator::{map, map_res, opt},
    sequence::{pair, tuple},
    IResult,
};

use nom::bytes::complete::{is_a, is_not};
use nom::character::complete::{alphanumeric1, line_ending, multispace0, space1};
use nom::combinator::recognize;
use nom::error::ErrorKind as NomErrorKind;
use nom::error::ParseError;
use nom::multi::many1;
use nom::sequence::delimited;

#[derive(Debug)]

pub enum Error {
    NomIncomplete(),
    NomError(String, NomErrorKind),
    NomFailure(String, NomErrorKind),
}

impl<'a> From<nom::Err<(&'a str, NomErrorKind)>> for Error {
    fn from(e: nom::Err<(&'a str, NomErrorKind)>) -> Error {
        match e {
            nom::Err::Incomplete(_) => Error::NomIncomplete(),
            nom::Err::Error((remaining, kind)) => Error::NomError(remaining.to_string(), kind),
            nom::Err::Failure((remaining, kind)) => Error::NomFailure(remaining.to_string(), kind),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

#[derive(Debug, PartialEq)]
pub enum SelectorType {
    SelectorList(Vec<String>), //, Option<String>)>),
    WildcardSelector(),
    NoSelector(),
}
#[derive(Debug, PartialEq)]
pub struct Import {
    pub line_number: u32,
    pub prefix_section: String,
    pub suffix: SelectorType,
}
impl Import {
    fn is_valid_import_segment_item(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    fn tuple_extractor<'a, E>() -> impl Fn(&'a str) -> IResult<&str, &str, E>
    where
        E: ParseError<&'a str>,
    {
        map(
            tuple((
                multispace0,
                alphanumeric1,
                multispace0,
                opt(tag(",")),
                multispace0,
            )),
            |e| e.1,
        )
    }
    fn consume_selector<'a, E>() -> impl Fn(&'a str) -> IResult<&str, SelectorType, E>
    where
        E: ParseError<&'a str>,
    {
        alt((
            map(
                tuple((
                    multispace0,
                    tag("."),
                    multispace0,
                    is_a("{"),
                    many1(map(Import::tuple_extractor(), |s| s.to_string())),
                    is_a("}"),
                    multispace0,
                )),
                |r| SelectorType::SelectorList(r.4),
            ),
            map(tuple((multispace0, tag("._"))), |_| {
                SelectorType::WildcardSelector()
            }),
            map(tuple((multispace0, is_not("."))), |_| {
                SelectorType::NoSelector()
            }),
        ))
    }

    pub fn parse_import(line_number: u32, input: &str) -> IResult<&str, Import> {
        let (input, _) = tuple((multispace0, tag("import"), space1, multispace0))(input)?;

        let (input, extracted) = recognize(many1(tuple((
            opt(tag(".")),
            alphanumeric1,
            nom::bytes::complete::take_while(Import::is_valid_import_segment_item),
        ))))(input)?;

        println!("Input: {:?}, extracted: {:?}", input, extracted);
        let (input, selector) = Import::consume_selector()(&input)?;

        Ok((
            input,
            Import {
                line_number: line_number,
                prefix_section: extracted.to_string(),
                suffix: selector,
            },
        ))
    }

    fn not_end_of_line(chr: char) -> bool {
        chr != '\n' && chr != '\r'
    }

    fn eat_till_end_of_line(input: &str) -> IResult<&str, (&str, &str)> {
        map(
            tuple((
                nom::bytes::complete::take_while(Import::not_end_of_line),
                nom::bytes::complete::take_while(|chr| chr == '\r'),
                nom::bytes::complete::take_while_m_n(0, 1, |chr| chr == '\n'),
            )),
            |r| (r.0, r.2),
        )(input)
    }
    pub fn parse(input: &str) -> Result<Vec<Import>> {
        let mut results_vec = Vec::new();
        let mut line_number = 0;
        let mut remaining_input = input;
        while remaining_input.len() > 3 {
            match Import::eat_till_end_of_line(remaining_input) {
                Ok((r, (current_line, end_of_line_eaten))) => {
                    if current_line.len() > 0 && current_line.contains("import") {
                        let (_, found) = Import::parse_import(line_number, remaining_input)?;
                        results_vec.push(found);
                    }

                    // println!(
                    //     "Inner loop: current_line: {:?} end_of_line_eaten: {:?}",
                    //     current_line, end_of_line_eaten
                    // );

                    // if we never found an end of line, must be end of file.
                    if end_of_line_eaten.len() > 0 {
                        remaining_input = r;
                    } else {
                        remaining_input = "";
                    }
                }
                Err(_) => {
                    remaining_input = "";
                }
            }
            line_number = line_number + 1;
        }

        Ok(results_vec)
    }
}

// #[derive(Debug, PartialEq)]
// pub struct ParsedFile {
//     pub package_name: Option<String>,
//     pub imports: Vec<Import>,
// }

// impl ParsedFile {

//     pub fn parse(input: &str) -> IResult<&str, &str> {
//         let merged_fn = ParsedFile::ws(alpha0);

//         merged_fn(&input)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    // #[test]
    // fn parse_header_line() {
    //     assert_eq!(ParsedFile::parse("foo bar baz"), Ok(("bar baz", "foo")));
    // }

    // #[test]
    // fn parse_simple_input() {
    //     let sample_input = "import com.twitter.scalding.RichDate";
    //     let expected_results = vec![Import {
    //         line_number: 0,
    //         prefix_section: "com.twitter.scalding.RichDate".to_string(),
    //         suffix: SelectorType::NoSelector(),
    //     }];

    //     let parsed_result = Import::parse(sample_input).unwrap();
    //     assert_eq!(parsed_result, expected_results);
    // }

    // #[test]
    // fn parse_multiple_lines_input() {
    //     let sample_input = "
    //     import com.twitter.scalding.RichDate
    //     import com.twitter.scalding.RichDate

    //     import com.twitter.scalding.RichDate
    //     ";
    //     let expected_results = vec![
    //         Import {
    //             line_number: 1,
    //             prefix_section: "com.twitter.scalding.RichDate".to_string(),
    //             suffix: SelectorType::NoSelector(),
    //         },
    //         Import {
    //             line_number: 2,
    //             prefix_section: "com.twitter.scalding.RichDate".to_string(),
    //             suffix: SelectorType::NoSelector(),
    //         },
    //         Import {
    //             line_number: 4,
    //             prefix_section: "com.twitter.scalding.RichDate".to_string(),
    //             suffix: SelectorType::NoSelector(),
    //         },
    //     ];

    //     let parsed_result = Import::parse(sample_input).unwrap();
    //     assert_eq!(parsed_result, expected_results);
    // }

    #[test]
    fn sub_sections() {
        let sample_input = "import com.twitter.scalding.{RichDate, DateOps}";
        let expected_results = vec![Import {
            line_number: 0,
            prefix_section: "com.twitter.scalding".to_string(),
            suffix: SelectorType::SelectorList(vec!["RichDate".to_string(), "DateOps".to_string()]),
        }];

        let parsed_result = Import::parse(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    #[test]
    fn test_wildcard() {
        let sample_input = "import com.twitter.scalding._";
        let expected_results = vec![Import {
            line_number: 0,
            prefix_section: "com.twitter.scalding".to_string(),
            suffix: SelectorType::WildcardSelector(),
        }];

        let parsed_result = Import::parse(sample_input).unwrap();
        assert_eq!(parsed_result, expected_results);
    }

    fn tuple_extractor_parser(i: &str) -> IResult<&str, &str> {
        Import::tuple_extractor()(i)
    }

    #[test]
    fn tuple_extractor() {
        let sample_input = "RichDate, DateOps";
        // let expected_results = vec![Import {
        //     line_number: 0,
        //     prefix_section: "com.twitter.scalding".to_string(),
        //     suffix: SelectorType::SelectorList(vec!["RichDate".to_string(), "DateOps".to_string()]),
        // }];

        let (remaining, parsed_result) = tuple_extractor_parser(sample_input).unwrap();
        assert_eq!(parsed_result, "RichDate");
        assert_eq!(remaining, "DateOps");

        let (remaining, parsed_result) = tuple_extractor_parser(remaining).unwrap();
        assert_eq!(parsed_result, "DateOps");
        assert_eq!(remaining, "");
    }

    //     fn parse_many_inputs {
    //         let sample_input = "
    //       import com.twitter.scalding.{DateOps, DateParser, RichDate}
    //       import org.foo.bar.baz._
    //       import zoop.noop.{asdf, pppp, _}

    //         import com.google.protobuf.Message
    //         ";

    //         let expected_results = vec![
    //             Import {

    //             }
    //         ]
    //   }
}
