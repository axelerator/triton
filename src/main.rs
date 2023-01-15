#![allow(unused)] // FIXME

extern crate combine;
mod layout;
mod sequence_diagram;

use itertools::Itertools;

use layout::Layout;
use svg::node::element::{Definitions, Group, Line, Marker, Polygon, Rectangle, Text};
use svg::node::Node;
use svg::Document;

use cassowary::strength::{MEDIUM, REQUIRED, STRONG, WEAK};
use cassowary::WeightedRelation::*;
use cassowary::{Expression, Solver, Variable};
use textwrap::{wrap, LineEnding, Options};

use crate::layout::BlockId;

use euclid::{self, Vector2D};

use sequence_diagram::parser::*;

type Scalar = f32;

pub struct ScreenSpace;

type Position = euclid::Vector2D<Scalar, ScreenSpace>;
type Size = euclid::Vector2D<Scalar, ScreenSpace>;

struct DiagramHead {
    label: Vec<String>,
    block: BlockId,
    participant_id: ParticipantId,
}

struct ParticipantLine {
    block: BlockId,
}

struct DiagramFooter {
    label: Vec<String>,
    block: BlockId,
    participant_id: ParticipantId,
}

struct MsgArrow {
    label: Vec<String>,
    direction: ArrowDirection,
    block: BlockId,
}

impl DiagramHead {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Vec<Box<dyn Node>> {
        let block = layout.b(self.block);
        let x = block.x_;
        let y = block.y_;
        let width = block.width_;
        let height = block.height_;
        let mut group = Group::new().set("transform", format!("translate({}, {})", x, y));

        let rect = Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", width)
            .set("height", height)
            .set("fill", "transparent")
            .set("stroke", "black")
            .set("stroke-width", 1);
        group = group.add(rect);
        for (i, line) in self.label.iter().enumerate() {
            let t = Text::new()
                .set("x", config.padding)
                .set("y", config.padding + ((i + 1) as f64) * config.line_height)
                .set("fill", "blue")
                .set("font-family", "monospace")
                .set("font-size", config.font_size)
                .add(svg::node::Text::new(line));
            group = group.add(t);
        }
        vec![Box::new(group)]
    }
}

impl DiagramFooter {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Vec<Box<dyn Node>> {
        let block = layout.b(self.block);
        let x = block.x_;
        let y = block.y_;
        let width = block.width_;
        let height = block.height_;
        let mut group = Group::new().set("transform", format!("translate({}, {})", x, y));

        let rect = Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", width)
            .set("height", height)
            .set("fill", "transparent")
            .set("stroke", "black")
            .set("stroke-width", 1);
        group = group.add(rect);
        for (i, line) in self.label.iter().enumerate() {
            let t = Text::new()
                .set("x", config.padding)
                .set("y", config.padding + ((i + 1) as f64) * config.line_height)
                .set("fill", "blue")
                .set("font-family", "monospace")
                .set("font-size", config.font_size)
                .add(svg::node::Text::new(line));
            group = group.add(t);
        }
        vec![Box::new(group)]
    }
}

impl ParticipantLine {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Vec<Box<dyn Node>> {
        let block = layout.b(self.block);
        let x = block.x_;
        let y = block.y_;
        let height = block.height_;
        let mut group = Group::new().set("transform", format!("translate({}, {})", x, y));

        let rect = Line::new()
            .set("x1", 0)
            .set("y1", 0)
            .set("x2", 0)
            .set("y2", height)
            .set("stroke", "black")
            .set("stroke-width", 1);
        group = group.add(rect);
        vec![Box::new(group)]
    }
}

const ARROW_TIP_LENGTH: f64 = 10.0;

impl MsgArrow {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Vec<Box<dyn Node>> {
        let block = layout.b(self.block);
        let x = block.x_;
        let y = block.y_;
        let width = block.width_;
        let height = block.height_;
        let mut group = Group::new().set("transform", format!("translate({}, {})", x, y));

        let rect = match self.direction {
            ArrowDirection::ToRight => Line::new()
                .set("x1", 0)
                .set("y1", height)
                .set("x2", width - ARROW_TIP_LENGTH)
                .set("y2", height)
                .set("stroke", "black")
                .set("stroke-width", 1)
                .set("marker-end", "url(#end-arrow)"),
            ArrowDirection::ToLeft => Line::new()
                .set("x1", 0)
                .set("y1", height)
                .set("x2", width)
                .set("y2", height)
                .set("stroke", "black")
                .set("stroke-width", 1)
                .set("marker-start", "url(#start-arrow)"),
        };
        group = group.add(rect);
        for (i, line) in self.label.iter().enumerate() {
            let t = Text::new()
                .set("x", 0)
                .set("y", (config.line_height * (i as f64)))
                .set("fill", "blue")
                .set("font-family", "monospace")
                .set("font-size", config.font_size)
                .add(svg::node::Text::new(line));

            group = group.add(t);
        }
        vec![Box::new(group)]
    }
}

struct SvgConfig {
    max_participant_head_length: usize,
    max_msg_label_length: usize,
    line_height: f64,
    letter_per_unit: f64,
    msg_gutter: f64,
    participant_gutter: f64,
    font_size: f64,
    canvas_size: Size,
    padding: f64,
}

const MAX_PARTICIPANT_HEAD_LENGTH: usize = 5;

fn to_svg(diagram: &SequenceDiagram, config: &SvgConfig) {
    let document = Document::new().set(
        "viewBox",
        (0, 0, config.canvas_size.x, config.canvas_size.y),
    );

    let mut layout = Layout::new();

    let mut heads = vec![];
    let mut footers = vec![];

    for participant in &diagram.participants {
        println!("Adding {}", participant.name);
        let b = layout.add_block();
        let lines = wrap(
            participant.name.as_str(),
            config.max_participant_head_length,
        );

        let longest_line = lines.iter().map(|l| l.len()).max().unwrap();
        let min_width: f64 =
            (longest_line as f64) / config.letter_per_unit + (2.0 * config.padding);
        layout.add_constraint(layout.b(b).width | EQ(WEAK) | min_width);

        let min_height = (lines.len() as f64) * config.line_height + (2.0 * config.padding);
        println!("{:?} min height: {:?}", participant.name, min_height);
        layout.add_constraint(layout.b(b).height | EQ(WEAK) | min_height);

        let label = lines.iter().map(|cow| cow.to_string()).collect();

        let head = DiagramHead {
            block: b,
            participant_id: participant.id,
            label,
        };
        heads.push(head);
    }

    let first_head = &heads[0];
    layout.add_constraint(layout.b(first_head.block).left() | EQ(REQUIRED) | 0.0);

    for (prev, next) in heads.iter().tuple_windows() {
        //println!("{} right LE than {} left", prev.label, next.label);
        layout.add_constraint(
            layout.b(prev.block).right() + config.participant_gutter
                | LE(REQUIRED)
                | layout.b(next.block).left(),
        );
        // align bottoms
        layout.add_constraint(
            layout.b(prev.block).bottom() | EQ(REQUIRED) | layout.b(next.block).bottom(),
        );
    }

    for head in &heads {
        let b = layout.add_block();

        layout.add_constraint(layout.b(b).width | EQ(REQUIRED) | layout.b(head.block).width);
        layout.add_constraint(layout.b(b).height | EQ(REQUIRED) | layout.b(head.block).height);
        layout.add_constraint(layout.b(b).left() | EQ(REQUIRED) | layout.b(head.block).left());

        let footer = DiagramFooter {
            block: b,
            participant_id: head.participant_id,
            label: head.label.clone(),
        };
        footers.push(footer);
    }

    for (prev, next) in footers.iter().tuple_windows() {
        // align tops
        layout
            .add_constraint(layout.b(prev.block).top() | EQ(REQUIRED) | layout.b(next.block).top());
    }

    let mut arrows = vec![];
    for m in &diagram.messages {
        let block = layout.add_block();
        let lines = wrap(m.msg.as_str(), config.max_msg_label_length);

        layout.add_constraint(
            layout.b(block).height | EQ(WEAK) | config.line_height * (lines.len() as f64),
        );
        let label = lines.iter().map(|cow| cow.to_string()).collect();

        let msg_arrow = MsgArrow {
            block,
            label,
            direction: m.direction.clone(),
        };

        layout.add_constraint(
            layout.b(block).top()
                | GE(REQUIRED)
                | layout.b(first_head.block).bottom() + config.msg_gutter,
        );

        let longest_line = lines.iter().map(|l| l.len()).max().unwrap();
        let min_width: f64 = (longest_line as f64) / config.letter_per_unit;
        layout.add_constraint(layout.b(block).width | GE(REQUIRED) | min_width as f64);

        let from = layout.b(heads
            .iter()
            .find(|h| h.participant_id == m.from)
            .unwrap()
            .block);
        // arrow should start in the middle of the "from" participant
        layout.add_constraint(
            layout.b(block).left() | EQ(REQUIRED) | from.left() + (from.width * 0.5),
        );

        let to = layout.b(heads
            .iter()
            .find(|h| h.participant_id == m.to)
            .unwrap()
            .block);
        layout
            .add_constraint(layout.b(block).right() | EQ(REQUIRED) | to.left() + (to.width * 0.5));

        arrows.push(msg_arrow);
    }

    for (prev, next) in arrows.iter().tuple_windows() {
        layout.add_constraint(
            layout.b(next.block).top()
                | GE(REQUIRED)
                | layout.b(prev.block).bottom() + config.msg_gutter,
        );
    }

    let last_arrow = arrows.last();
    if let Some(arrow) = last_arrow {
        let first_footer = &footers[0];
        layout.add_constraint(
            layout.b(first_footer.block).top()
                | EQ(REQUIRED)
                | layout.b(arrow.block).bottom() + config.msg_gutter,
        );
    }

    let mut participant_lines = vec![];
    for (head, footer) in heads.iter().zip(footers.iter()) {
        let block_id = layout.add_block();

        let block = layout.b(block_id);
        layout.add_constraint(block.top() | EQ(REQUIRED) | layout.b(head.block).bottom());

        let block = layout.b(block_id);
        layout.add_constraint(block.bottom() | EQ(REQUIRED) | layout.b(footer.block).top());

        let block = layout.b(block_id);
        layout.add_constraint(
            block.left()
                | EQ(REQUIRED)
                | layout.b(head.block).left() + (layout.b(head.block).width * 0.5),
        );

        participant_lines.push(ParticipantLine { block: block_id });
    }

    layout.solve();

    let mut doc = document;

    let defs = Definitions::new()
        .add(
            Marker::new()
                .set("id", "start-arrow")
                .set("markerWidth", "10")
                .set("markerHeight", "7")
                .set("refX", 0)
                .set("refY", 3.5)
                .set("orient", "auto")
                .add(Polygon::new().set("points", "10 0, 10 7, 0 3.5")),
        )
        .add(
            Marker::new()
                .set("id", "end-arrow")
                .set("markerWidth", "10")
                .set("markerHeight", "7")
                .set("refX", 0)
                .set("refY", 3.5)
                .set("orient", "auto")
                .add(Polygon::new().set("points", "0 0, 10 3.5, 0 7")),
        );

    doc = doc.add(defs);

    for elem in heads {
        for e in elem.to_svg(&layout, config) {
            doc = doc.add(e);
        }
    }

    for elem in footers {
        for e in elem.to_svg(&layout, config) {
            doc = doc.add(e);
        }
    }
    for elem in arrows {
        for e in elem.to_svg(&layout, config) {
            doc = doc.add(e);
        }
    }

    for elem in participant_lines {
        for e in elem.to_svg(&layout, config) {
            doc = doc.add(e);
        }
    }

    svg::save("image.svg", &doc).unwrap();
}

fn main() {
    let svg_config = SvgConfig {
        max_participant_head_length: 5,
        max_msg_label_length: 60,
        line_height: 10.0,
        letter_per_unit: 0.11,
        msg_gutter: 20.0,
        participant_gutter: 20.0,
        font_size: 12.0,
        canvas_size: Vector2D::new(400.0, 400.0),
        padding: 4.0,
    };

    match sequence_diagram::parser::parse(
        "Alice->Bob long name: hello, how are today my dear?\nJohn->Bob long name: How dat?\nBob long name->John: It's Alice\nBob long name->Alice: I'm fine\n".to_string(),
    ) {
        Some((diagram)) => {
            to_svg(&diagram, &svg_config);
        }
        None => {
            println!("none");
        }
    }
}
