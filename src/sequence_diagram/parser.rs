use std::collections::HashMap;

use combine::error::StringStreamError;
use combine::parser::char::{char, newline, space, string};
use combine::parser::choice::choice;
use combine::parser::repeat::take_until;
use combine::{
    many1, none_of, optional, sep_by1, skip_many, skip_many1, ParseError, Parser, Stream,
};

pub type ParticipantId = usize;
pub type MessageId = usize;

#[derive(PartialEq, Debug)]
pub struct Participant {
    pub id: ParticipantId,
    pub name: String,
}

#[derive(PartialEq, Debug)]
pub struct Activation {
    pub participant_id: ParticipantId,
    pub from: MessageId,
    pub to: MessageId,
    pub level: u16,
}

#[derive(PartialEq, Debug)]
pub struct Message {
    pub id: MessageId,
    pub left: ParticipantId,
    pub right: ParticipantId,
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
    pub activations: Vec<Activation>,
}

#[derive(PartialEq, Debug)]
pub enum ActivationChange {
    Activate,
    Deactivate,
}

#[derive(PartialEq, Debug)]
pub struct MessageLine {
    pub from: String,
    pub arrow: Arrow,
    pub to: String,
    pub msg: String,
    pub activation: Option<ActivationChange>,
}

#[derive(PartialEq, Debug)]
pub enum Line {
    Empty,
    Message(MessageLine),
    Participant(String),
    Actor(String),
    Note(NotePosition, String),
}

#[derive(PartialEq, Debug, Clone)]
pub enum Arrow {
    DottedNoArrow,
    SolidNoArrow,
}

pub fn parse(src: String) -> Result<SequenceDiagram, StringStreamError> {
    let mut parser = lines_parser();
    match parser.parse(format!("{src}\n").as_str()) {
        Ok((lines, _)) => Ok(build_diagram(lines)),
        Err(e) => Err(e),
    }
}

fn build_participants(lines: &Vec<Line>) -> Vec<Participant> {
    let mut participant_names = vec![];
    let mut participants: Vec<Participant> = vec![];
    for line in lines {
        match line {
            Line::Message(MessageLine { from, to, .. }) => {
                if !participant_names.contains(from) {
                    let p: Participant = Participant {
                        name: from.clone(),
                        id: participants.len(),
                    };
                    participant_names.push(from.clone());
                    participants.push(p);
                }
                if !participant_names.contains(to) {
                    let p: Participant = Participant {
                        name: to.clone(),
                        id: participants.len(),
                    };
                    participant_names.push(to.clone());
                    participants.push(p);
                }
            }
            Line::Participant(name) => {
                if !participant_names.contains(name) {
                    let p: Participant = Participant {
                        name: name.clone(),
                        id: participants.len(),
                    };
                    participant_names.push(name.clone());
                    participants.push(p);
                }
            }
            Line::Actor(name) => {
                if !participant_names.contains(name) {
                    let p: Participant = Participant {
                        name: name.clone(),
                        id: participants.len(),
                    };
                    participant_names.push(name.clone());
                    participants.push(p);
                }
            }

            _ => {}
        }
    }
    participants
}

fn build_diagram(lines: Vec<Line>) -> SequenceDiagram {
    let mut msg_lines: Vec<&MessageLine> = vec![];
    for line in &lines {
        if let Line::Message(msg_line) = line {
            msg_lines.push(msg_line);
        }
    }

    let participants = build_participants(&lines);

    let mut activations: Vec<Activation> = vec![];
    // stores the level and start of the last activation of each participant
    let mut open_activations: HashMap<ParticipantId, Vec<(u16, MessageId)>> = HashMap::new();
    let mut messages = vec![];
    for (line_nr, line) in msg_lines.iter().enumerate() {
        let from_idx = participants.iter().find(|&p| p.name == line.from);
        let to_idx = participants.iter().find(|&p| p.name == line.to);
        match (from_idx, to_idx) {
            (Some(from), Some(to)) => {
                let msg = if from.id > to.id {
                    Message {
                        id: line_nr,
                        left: to.id,
                        right: from.id,
                        msg: line.msg.clone(),
                        arrow: line.arrow.clone(),
                        direction: ArrowDirection::ToLeft,
                    }
                } else {
                    Message {
                        id: line_nr,
                        left: from.id,
                        right: to.id,
                        msg: line.msg.clone(),
                        arrow: line.arrow.clone(),
                        direction: ArrowDirection::ToRight,
                    }
                };
                if let Some(activation_change) = &line.activation {
                    match activation_change {
                        ActivationChange::Activate => {
                            let open: &mut Vec<(u16, MessageId)> =
                                open_activations.entry(to.id).or_insert(vec![]);
                            let (last_level, _) = open.last().unwrap_or(&(0, 0));
                            open.push((last_level + 1, line_nr));
                        }
                        ActivationChange::Deactivate => {
                            let open: &mut Vec<(u16, MessageId)> = open_activations
                                .get_mut(&from.id)
                                .expect("Wasn't activated");
                            let (level, message_id) = open.last().expect("Still wasn't activated");

                            let activation = Activation {
                                participant_id: from.id,
                                from: *message_id,
                                to: line_nr,
                                level: *level,
                            };
                            open.pop();
                            activations.push(activation);
                        }
                    }
                }
                messages.push(msg);
            }
            _ => {
                println!("NO {:?}, {:?}", from_idx, line.to);
            }
        }
    }
    // so lowest level activations come first
    activations.reverse();
    SequenceDiagram {
        participants,
        messages,
        activations,
    }
}

fn arrow_parser<Input>() -> impl Parser<Input, Output = Arrow>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let dotted_no_arrow_parser = string("-->").map(|_| Arrow::DottedNoArrow);
    let solid_no_arrow_parser = string("->").map(|_| Arrow::SolidNoArrow);

    choice((solid_no_arrow_parser, dotted_no_arrow_parser))
}

fn activation_parser<Input>() -> impl Parser<Input, Output = Option<ActivationChange>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let activate = string("+").map(|_| ActivationChange::Activate);
    let deactivate = string("-").map(|_| ActivationChange::Deactivate);

    optional(choice((deactivate, activate)))
}

fn sender_name_parser<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    take_until(char('-'))
}

fn receiver_name_parser<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (skip_many(space()), take_until(char(':'))).map(|(_, s)| s)
}

fn empty_line_parser<Input>() -> impl Parser<Input, Output = Line>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (newline()).map(|_| Line::Empty)
}

fn participant_line_parser<Input>() -> impl Parser<Input, Output = Line>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        string("participant"),
        skip_many(char(' ')),
        take_until::<String, Input, combine::parser::token::Token<Input>>(char('\n')),
    )
        .map(|(_, _, name)| Line::Participant(name.trim().to_string()))
}

fn actor_line_parser<Input>() -> impl Parser<Input, Output = Line>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        string("actor"),
        skip_many(char(' ')),
        take_until::<String, Input, combine::parser::token::Token<Input>>(char('\n')),
    )
        .map(|(_, _, name)| Line::Actor(name.trim().to_string()))
}

#[derive(PartialEq, Debug)]
pub enum NotePosition {
    LeftOf(String),
    Over(Vec<String>),
    RightOf(String),
}

fn left_of_parser<Input>() -> impl Parser<Input, Output = NotePosition>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        string("left of"),
        skip_many1(char(' ')),
        take_until(char(':')),
    )
        .map(|(_, _, name)| NotePosition::LeftOf(name))
}

fn right_of_parser<Input>() -> impl Parser<Input, Output = NotePosition>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        string("left of"),
        skip_many1(char(' ')),
        take_until(char(':')),
    )
        .map(|(_, _, name)| NotePosition::RightOf(name))
}

fn over_parser<Input>() -> impl Parser<Input, Output = NotePosition>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let participant_name = many1(none_of(":,".chars()));
    let separator = char(',');
    (
        string("over"),
        skip_many1(char(' ')),
        sep_by1(participant_name, separator),
    )
        .map(|(_, _, names): (&str, (), Vec<String>)| {
            NotePosition::Over(names.iter().map(|s| s.trim().to_string()).collect())
        })
}

fn note_position_parser<Input>() -> impl Parser<Input, Output = NotePosition>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    choice((left_of_parser(), right_of_parser(), over_parser()))
}

fn note_line_parser<Input>() -> impl Parser<Input, Output = Line>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        string("Note"),
        skip_many1(char(' ')),
        note_position_parser(),
        char(':'),
        skip_many(char(' ')),
        take_until(char('\n')),
    )
        .map(|(_, _, note_position, _, _, msg)| Line::Note(note_position, msg))
}

fn msg_line_parser<Input>() -> impl Parser<Input, Output = Line>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let colon = char(':');
    let until_newline = take_until(char('\n'));
    let eol = char('\n');

    (
        sender_name_parser(),
        arrow_parser(),
        activation_parser(),
        receiver_name_parser(),
        colon,
        skip_many(char(' ')),
        until_newline,
        eol,
    )
        .map(|(from, arrow, activation, to, _, _, msg, _)| {
            Line::Message(MessageLine {
                from,
                arrow,
                activation,
                to,
                msg,
            })
        })
}

fn line_parser<Input>() -> impl Parser<Input, Output = Line>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (
        skip_many(char(' ')),
        choice((
            empty_line_parser(),
            note_line_parser(),
            participant_line_parser(),
            actor_line_parser(),
            msg_line_parser(),
        )),
    )
        .map(|(_, l)| l)
}

fn lines_parser<Input>() -> impl Parser<Input, Output = Vec<Line>>
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
                id: 0,
                left: 0,
                right: 1,
                msg: "hello".to_string(),
                arrow: Arrow::SolidNoArrow,
                direction: ArrowDirection::ToRight,
            }],
            activations: vec![],
        };
        let input = "Alice->Bob: hello\n";
        assert_eq!(parse(input.to_string()), Ok(expected));
    }

    #[test]
    fn test_sender_name_parser() {
        assert_eq!(
            Ok(("Alice".to_string(), "-")),
            sender_name_parser().parse("Alice-")
        );
    }

    #[test]
    fn test_participant_line_parser() {
        let expected = |n: &str| -> Result<(Line, &str), StringStreamError> {
            Ok((Line::Participant(n.to_string()), "\n"))
        };
        assert_eq!(
            expected("Alice"),
            line_parser().parse("participant Alice\n")
        );

        assert_eq!(
            expected("Alice"),
            line_parser().parse("participant  Alice\n")
        );

        assert_eq!(
            expected("Alice"),
            line_parser().parse(" participant Alice\n")
        );

        assert_eq!(
            expected("Alice"),
            line_parser().parse("participant Alice \n")
        );

        assert_eq!(expected("part"), line_parser().parse("participant part\n"));
    }

    #[test]
    fn test_message_line_parser() {
        let expected = Line::Message(MessageLine {
            from: "Alice".to_string(),
            arrow: Arrow::SolidNoArrow,
            to: "Bob".to_string(),
            msg: "hello".to_string(),
            activation: None,
        });
        assert_eq!(
            Ok((expected, "")),
            line_parser().parse(" Alice->Bob:hello\n")
        );
    }

    #[test]
    fn test_message_line_parser_with_space() {
        let expected = Line::Message(MessageLine {
            from: "Alice".to_string(),
            arrow: Arrow::SolidNoArrow,
            to: "Bob".to_string(),
            msg: "hello".to_string(),
            activation: None,
        });
        assert_eq!(
            Ok((expected, "")),
            line_parser().parse("Alice->Bob: hello\n")
        );
    }

    #[test]
    fn test_line_parser() {
        let expected = Line::Message(MessageLine {
            from: "Alice".to_string(),
            arrow: Arrow::SolidNoArrow,
            to: "Bob".to_string(),
            msg: "hello".to_string(),
            activation: None,
        });
        assert_eq!(
            Ok((expected, "")),
            line_parser().parse(" Alice->Bob:hello\n")
        );
    }

    #[test]
    fn test_note_over() {
        let expected = Line::Note(
            NotePosition::Over(vec!["Alice".to_string(), "Bob".to_string()]),
            "Foo".to_string(),
        );
        assert_eq!(
            Ok((expected, "\n")),
            line_parser().parse("Note over Alice , Bob:Foo\n")
        );
    }

    #[test]
    fn test_over_parser() {
        let expected = NotePosition::Over(vec!["Alice".to_string(), "Bob".to_string()]);

        assert_eq!(Ok((expected, ":")), over_parser().parse("over Alice,Bob:"));
    }

    #[test]
    fn test_note_left_of_parser() {
        let expected = Line::Note(NotePosition::LeftOf("Alice".to_string()), "Foo".to_string());

        assert_eq!(
            Ok((expected, "\n")),
            note_line_parser().parse("Note left of Alice: Foo\n")
        );
    }

    #[test]
    fn test_note_pos_parser() {
        let expected = NotePosition::LeftOf("Alice".to_string());

        assert_eq!(
            Ok((expected, ":")),
            note_position_parser().parse("left of Alice:")
        );
    }

    #[test]
    fn test_line_parser_with_empty_line() {
        let expected = Line::Empty;
        assert_eq!(Ok((expected, "")), line_parser().parse(" \n"));
    }

    #[test]
    fn test_whitspace_single_message() {
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
                id: 0,
                left: 0,
                right: 1,
                msg: "hello".to_string(),
                arrow: Arrow::SolidNoArrow,
                direction: ArrowDirection::ToRight,
            }],
            activations: vec![],
        };
        let input = " Alice->Bob:hello\n";
        assert_eq!(parse(input.to_string()), Ok(expected));
    }

    #[test]
    fn test_roundtrip() {
        let input = r#"
            Alice->Bob:How are you?
            Bob->Alice:I'm fine!
        "#;

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
            messages: vec![
                Message {
                    id: 0,
                    left: 0,
                    right: 1,
                    msg: "How are you?".to_string(),
                    arrow: Arrow::SolidNoArrow,
                    direction: ArrowDirection::ToRight,
                },
                Message {
                    id: 1,
                    left: 0,
                    right: 1,
                    msg: "I'm fine!".to_string(),
                    arrow: Arrow::SolidNoArrow,
                    direction: ArrowDirection::ToLeft,
                },
            ],
            activations: vec![],
        };
        assert_eq!(parse(input.to_string()), Ok(expected));
    }
}
