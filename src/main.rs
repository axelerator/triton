extern crate combine;
use combine::parser::char::{char, letter, space, string};
use combine::parser::choice::choice;
use combine::parser::repeat::{take_until};
use combine::{many1, skip_many, ParseError, Parser, Stream};

use svg::node::element::{Line, Rectangle, Text};
use svg::node::Node;
use svg::Document;

use euclid;

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

type Scalar = f32;

pub struct ScreenSpace;

type Position = euclid::Vector2D<Scalar, ScreenSpace>;

pub enum DiagramElement {
    ParticipantLine { x: Scalar },
    ParticipantHead { pos: Position, label: String, width: Scalar, height: Scalar }
}

impl DiagramElement {
    fn to_svg(&self) -> Vec<Box<dyn Node>> {
        match self {
            DiagramElement::ParticipantLine{x} => {
                let l = Line::new()
                    .set("x1", *x)
                    .set("y1", 10.0)
                    .set("x2", *x)
                    .set("y2", 20.0)
                    .set("stroke", "black")
                    .set("stroke-width",1);
                vec![Box::new(l)]
            }
            DiagramElement::ParticipantHead{ pos, width, height, label } => {
                let x = pos.x - (width * 0.5);
                let y = pos.y - (height * 0.5);
                let rect = Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", *width)
                    .set("height", *height)
                    .set("fill", "transparent")
                    .set("stroke", "black")
                    .set("stroke-width",1);
                let text = Text::new()
                    .set("x", x)
                    .set("y", height * 0.5)
                    .set("width", *width)
                    .set("height", *height)
                    .set("fill", "blue")
                    .set("font-family", "monospace")
                    .set("font-size", 2)
                    .add(svg::node::Text::new(label.clone()));
                vec![Box::new(rect), Box::new(text)]
            }
        }
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

fn line_parser<Input>() -> impl Parser<Input, Output = MessageLine>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    // Construct a parser that parses *many* (and at least *1) *letter*s
    let word_left = many1(letter());
    let word_right = many1(letter());
    let colon = char(':');
    let until_newline = take_until(char('\n'));
    let eol = char('\n');

    (
        skip_many(space()),
        word_left,
        arrow_parser(),
        word_right,
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
                messages.push(Message {
                    from: from.id,
                    to: to.id,
                    msg: line.msg.clone(),
                    arrow: line.arrow.clone(),
                });
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

fn to_svg(diagram: &SequenceDiagram){
    let document = Document::new().set("viewBox", (0, 0, 70, 70));


    let mut elements = vec![];
    let mut x = 0.0;
    for participant in &diagram.participants {
        let line = DiagramElement::ParticipantLine { x };
        elements.push(line);
        let head = DiagramElement::ParticipantHead {
            pos: Position::new(x, 5.0),
            label: participant.name.clone(),
            width: 10.0,
            height: 10.0,
        };
        elements.push(head);
        x += 10.0;
    }

    let mut doc = document;
    for elem in elements {
        for e in elem.to_svg() {
            doc = doc.add(e);
        }
    }

    svg::save("image.svg", &doc).unwrap();
}

fn main() {
    let mut parser = lines_parser();

    match parser.parse("Alice->Bob: hello\nJohn->Bob: foo\n") {
        Ok((lines, _)) => {
            // println!("Result {:?}", build_diagram(lines));
            to_svg(&build_diagram(lines));
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
