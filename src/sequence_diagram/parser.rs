use combine::parser::char::{char, letter, space, spaces, string};
use combine::parser::choice::choice;
use combine::parser::repeat::take_until;
use combine::{many1, skip_many, ParseError, Parser, Stream};

// sequenceDiagram
//    Alice->>John: Hello John, how are you?
//    John-->>Alice: Great!
//    Alice-)John: See you later!

pub type ParticipantId = usize;

#[derive(PartialEq, Debug)]
pub struct Participant {
    pub id: ParticipantId,
    pub name: String,
}

#[derive(PartialEq, Debug)]
pub struct Message {
    pub from: ParticipantId,
    pub to: ParticipantId,
    pub msg: String,
    pub arrow: Arrow,
    pub direction: ArrowDirection,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ArrowDirection {
    ToRight,
    ToLeft,
}

#[derive(PartialEq, Debug)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
}

#[derive(PartialEq, Debug)]
pub struct MessageLine {
    pub from: String,
    pub arrow: Arrow,
    pub to: String,
    pub msg: String,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Arrow {
    DottedNoArrow,
    SolidNoArrow,
}

pub fn parse(src: String) -> Option<SequenceDiagram> {
    let mut parser = lines_parser();
    match parser.parse(src.as_str()) {
        Ok((lines, _)) => Some(build_diagram(lines)),
        Err(e) => {
            println!("Error: {:?}", e);
            None
        }
    }
}

fn build_diagram(lines: Vec<MessageLine>) -> SequenceDiagram {
    let mut participant_names = vec![];
    let mut participants: Vec<Participant> = vec![];
    let mut id = 0;
    for line in &lines {
        if !participant_names.contains(&line.from) {
            let p: Participant = Participant {
                name: line.from.clone(),
                id,
            };
            participant_names.push(line.from.clone());
            participants.push(p);
            id += 1;
        }
        if !participant_names.contains(&line.to) {
            let p: Participant = Participant {
                name: line.to.clone(),
                id,
            };
            participant_names.push(line.to.clone());
            participants.push(p);
            id += 1;
        }
    }

    let mut messages = vec![];
    for line in &lines {
        let from_idx = participants.iter().find(|&p| p.name == line.from);
        let to_idx = participants.iter().find(|&p| p.name == line.to);
        match (from_idx, to_idx) {
            (Some(from), Some(to)) => {
                let msg = if from.id > to.id {
                    Message {
                        from: to.id,
                        to: from.id,
                        msg: line.msg.clone(),
                        arrow: line.arrow.clone(),
                        direction: ArrowDirection::ToLeft,
                    }
                } else {
                    Message {
                        from: from.id,
                        to: to.id,
                        msg: line.msg.clone(),
                        arrow: line.arrow.clone(),
                        direction: ArrowDirection::ToRight,
                    }
                };
                messages.push(msg);
            }
            _ => {
                println!("NO {:?}, {:?}", from_idx, line.to);
            }
        }
    }

    SequenceDiagram {
        participants,
        messages,
    }
}

fn arrow_parser<Input>() -> impl Parser<Input, Output = Arrow>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let dotted_no_arrow_parser = string("-->").map(|_| Arrow::DottedNoArrow);
    let solid_no_arrow_parser = string("->").map(|_| Arrow::SolidNoArrow);

    choice((solid_no_arrow_parser, dotted_no_arrow_parser))
}

fn letters_with_spaces<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(letter().or(space())).map(|s: String| s)
}

fn line_parser<Input>() -> impl Parser<Input, Output = MessageLine>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    // Construct a parser that parses *many* (and at least *1) *letter*s
    let colon = char(':');
    let until_newline = take_until(char('\n'));
    let eol = char('\n');

    (
        skip_many(space()),
        letters_with_spaces(),
        arrow_parser(),
        letters_with_spaces(),
        colon,
        until_newline,
        eol,
    )
        .map(|(_, from, arrow, to, _, msg, _)| MessageLine {
            from,
            arrow,
            to,
            msg,
        })
}

fn lines_parser<Input>() -> impl Parser<Input, Output = Vec<MessageLine>>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(line_parser())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_message() {
        let expected = SequenceDiagram {
            participants: vec![
                Participant {
                    id: 0,
                    name: "Alice".to_string(),
                },
                Participant {
                    id: 1,
                    name: "Bob".to_string(),
                },
            ],
            messages: vec![Message {
                from: 0,
                to: 1,
                msg: " hello".to_string(),
                arrow: Arrow::SolidNoArrow,
            }],
        };
        let input = "Alice->Bob: hello\n";
        assert_eq!(parse(input.to_string()), Some(expected));
    }
}
