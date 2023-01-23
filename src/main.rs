#![allow(unused)] // FIXME

extern crate combine;
mod layout;
mod sequence_diagram;

use itertools::Itertools;

use layout::Layout;
use svg::node::element::{Definitions, Group, Line, Marker, Polygon, Rectangle, Style, Text};
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

struct ParticipantMarker {
    lines: Vec<String>,
    block_id: BlockId,
    participant_id: ParticipantId,
}

struct ParticipantLine {
    block: BlockId,
    participant_id: ParticipantId,
}

struct MsgArrow {
    msg_id: MessageId,
    label: Vec<String>,
    direction: ArrowDirection,
    left: ParticipantId,
    right: ParticipantId,
    block: BlockId,
}

struct ActivationMarker {
    block: BlockId,
    level: u16,
}

impl ParticipantMarker {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Group {
        let block = layout.b(self.block_id);
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
            .set("rx", config.corner_radius)
            .set("stroke-width", 1);
        group = group.add(rect);
        for (i, line) in self.lines.iter().enumerate() {
            let t = Text::new()
                .set("x", config.padding)
                .set("y", config.padding + ((i + 1) as f64) * block.line_height)
                .set("fill", "blue")
                .set("font-family", "monospace")
                .set("font-size", config.font_size)
                .add(svg::node::Text::new(line));
            group = group.add(t);
        }
        group
    }
}

impl ParticipantLine {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Group {
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
        group
    }
}

const ARROW_TIP_LENGTH: f64 = 10.0;

impl MsgArrow {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Group {
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
        let text_height = (self.label.len() as f64) * block.line_height;
        for (i, line) in self.label.iter().enumerate() {
            let t = Text::new()
                .set("x", config.padding)
                .set(
                    "y",
                    (config.padding + text_height) - (i as f64) * block.line_height,
                )
                .set("fill", "blue")
                .set("font-family", "monospace")
                .set("font-size", config.font_size)
                .add(svg::node::Text::new(line));

            group = group.add(t);
        }
        group
    }
}

impl ActivationMarker {
    fn to_svg(&self, layout: &Layout, config: &SvgConfig) -> Group {
        let block = layout.b(self.block);
        let x = block.x_;
        let y = block.y_;
        let mut group = Group::new().set("transform", format!("translate({}, {})", x, y));

        let rect = Rectangle::new()
            .set("x", 0)
            .set("y", 0)
            .set("width", block.width_)
            .set("height", block.height_)
            .set("fill", "gray")
            .set("stroke", "#333")
            .set("stroke-width", 1);
        group = group.add(rect);
        group
    }
}

struct SvgConfig {
    max_participant_head_length: usize,
    max_msg_label_length: usize,
    letter_per_unit: f64,
    msg_gutter: f64,
    participant_gutter: f64,
    font_size: f64,
    padding: f64,
    corner_radius: f64,
}

enum ArrowSide {
    Unknown,
    Left(BlockId),
    Right(BlockId),
}

fn to_svg(diagram: &SequenceDiagram, config: &SvgConfig) {
    let mut layout = Layout::new();
    let mut arrows = vec![];
    for m in &diagram.messages {
        let (block, lines) =
            layout.add_text_block(&m.msg, config.max_msg_label_length, config.padding);

        let msg_arrow = MsgArrow {
            msg_id: m.id,
            block,
            label: lines,
            direction: m.direction.clone(),
            left: m.left,
            right: m.right,
        };
        arrows.push(msg_arrow);
    }

    layout.distribute(
        layout::Orientation::Vertical,
        config.msg_gutter,
        arrows.iter().map(|a| &a.block),
    );

    for participant in &diagram.participants {
        let arrows_for_participant = arrows
            .iter()
            .filter(|a| a.left == participant.id || a.right == participant.id);
        let mut first = ArrowSide::Unknown;
        for arrow in arrows_for_participant {
            match first {
                ArrowSide::Unknown => {
                    first = if arrow.left == participant.id {
                        ArrowSide::Left(arrow.block)
                    } else {
                        ArrowSide::Right(arrow.block)
                    };
                }
                ArrowSide::Left(first_block_id) => {
                    if arrow.left == participant.id {
                        layout.add_constraint(
                            layout.b(arrow.block).left()
                                | EQ(REQUIRED)
                                | layout.b(first_block_id).left(),
                        );
                    } else {
                        layout.add_constraint(
                            layout.b(arrow.block).right()
                                | EQ(REQUIRED)
                                | layout.b(first_block_id).left(),
                        );
                    };
                }
                ArrowSide::Right(first_block_id) => {
                    if arrow.left == participant.id {
                        layout.add_constraint(
                            layout.b(arrow.block).left()
                                | EQ(REQUIRED)
                                | layout.b(first_block_id).right(),
                        );
                    } else {
                        layout.add_constraint(
                            layout.b(arrow.block).right()
                                | EQ(REQUIRED)
                                | layout.b(first_block_id).right(),
                        );
                    };
                }
            }
        }
    }

    let mut participant_lines: Vec<ParticipantLine> = vec![];
    if let (Some(first_arrow), Some(last_arrow)) = (arrows.first(), arrows.last()) {
        let mut last_block = None;
        for participant in &diagram.participants {
            let block_id = layout.add_block();

            let block = layout.b(block_id);
            layout.add_constraint(
                block.top() | EQ(REQUIRED) | layout.b(first_arrow.block).top() - config.msg_gutter,
            );
            let block = layout.b(block_id);
            layout.add_constraint(
                block.bottom()
                    | EQ(REQUIRED)
                    | layout.b(last_arrow.block).bottom() + config.msg_gutter,
            );
            if let Some(prev_block_id) = last_block {
                let block = layout.b(block_id);
                layout.add_constraint(block.top() | EQ(REQUIRED) | layout.b(prev_block_id).top());
                let block = layout.b(block_id);
                layout.add_constraint(
                    block.bottom() | EQ(REQUIRED) | layout.b(prev_block_id).bottom(),
                );
            }

            last_block = Some(block_id);
            participant_lines.push(ParticipantLine {
                block: block_id,
                participant_id: participant.id,
            });
        }
    }

    let mut activation_markers = vec![];
    for activation in &diagram.activations {
        if let (Some(from), Some(to), Some(p_line)) = (
            arrows.iter().find(|a| a.msg_id == activation.from),
            arrows.iter().find(|a| a.msg_id == activation.to),
            participant_lines
                .iter()
                .find(|l| l.participant_id == activation.participant_id),
        ) {
            let block_id = layout.add_block();

            layout.add_constraint(
                layout.b(block_id).top() | EQ(REQUIRED) | layout.b(from.block).bottom(),
            );
            layout.add_constraint(
                layout.b(block_id).bottom() | EQ(REQUIRED) | layout.b(to.block).bottom(),
            );

            layout.add_constraint(layout.b(block_id).width | EQ(REQUIRED) | layout.glyphs_height);

            layout.add_constraint(
                layout.b(block_id).left() + (layout.glyphs_height) * 1.5
                    - (activation.level as f64 * (layout.glyphs_height * 0.5))
                    | EQ(REQUIRED)
                    | layout.b(p_line.block).left(),
            );

            activation_markers.push(ActivationMarker {
                block: block_id,
                level: activation.level,
            });
        }
    }

    for participant in &diagram.participants {
        let arrows_for_participant = arrows
            .iter()
            .filter(|a| a.left == participant.id || a.right == participant.id);
        let participant_line = participant_lines
            .iter()
            .find(|pl| pl.participant_id == participant.id)
            .unwrap();
        let mut first = ArrowSide::Unknown;
        for arrow in arrows_for_participant {
            match first {
                ArrowSide::Unknown => {
                    first = if arrow.left == participant.id {
                        layout.add_constraint(
                            layout.b(arrow.block).left()
                                | EQ(REQUIRED)
                                | layout.b(participant_line.block).left(),
                        );
                        ArrowSide::Left(arrow.block)
                    } else {
                        layout.add_constraint(
                            layout.b(arrow.block).right()
                                | EQ(REQUIRED)
                                | layout.b(participant_line.block).left(),
                        );
                        ArrowSide::Right(arrow.block)
                    };
                }
                ArrowSide::Left(first_block_id) => {}
                ArrowSide::Right(first_block_id) => {}
            }
        }
    }

    let mut heads: Vec<ParticipantMarker> = vec![];
    let mut footers: Vec<ParticipantMarker> = vec![];

    for participant in &diagram.participants {
        let participant_line = participant_lines
            .iter()
            .find(|pl| pl.participant_id == participant.id)
            .unwrap();
        let (b, lines) = layout.add_text_block(
            &participant.name,
            config.max_participant_head_length,
            config.padding,
        );
        let head = ParticipantMarker {
            block_id: b,
            participant_id: participant.id,
            lines,
        };
        layout.add_constraint(
            layout.b(b).bottom() | EQ(REQUIRED) | layout.b(participant_line.block).top(),
        );
        layout.add_constraint(
            layout.b(b).left() + (layout.b(b).width * 0.5)
                | EQ(REQUIRED)
                | layout.b(participant_line.block).left(),
        );
        heads.push(head);

        let (footer_b, lines) = layout.add_text_block(
            &participant.name,
            config.max_participant_head_length,
            config.padding,
        );
        let footer = ParticipantMarker {
            block_id: footer_b,
            participant_id: participant.id,
            lines,
        };
        layout.add_constraint(
            layout.b(footer_b).top() | EQ(REQUIRED) | layout.b(participant_line.block).bottom(),
        );
        layout.add_constraint(
            layout.b(footer_b).left() + (layout.b(footer_b).width * 0.5)
                | EQ(REQUIRED)
                | layout.b(participant_line.block).left(),
        );
        footers.push(footer);
    }

    layout.solve();
    let mut doc = Document::new().set("viewBox", (0, 0, layout.width(), layout.height()));
    let defs = Definitions::new()
        .add(Style::new("@font-face { font-family: Roboto-Regular; src: url(\"resources/fonts/Roboto-Regular.ttf\") }"))
        .add(Style::new("text {font-family:Roboto-Regular,Roboto;}"))
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
        doc = doc.add(elem.to_svg(&layout, config));
    }

    for elem in footers {
        doc = doc.add(elem.to_svg(&layout, config));
    }

    for elem in participant_lines {
        doc = doc.add(elem.to_svg(&layout, config));
    }

    for elem in activation_markers {
        doc = doc.add(elem.to_svg(&layout, config));
    }

    for elem in arrows {
        doc = doc.add(elem.to_svg(&layout, config));
    }
    svg::save("image.svg", &doc).unwrap();
}

fn main() {
    let svg_config = SvgConfig {
        max_participant_head_length: 5,
        max_msg_label_length: 60,
        letter_per_unit: 0.25,
        msg_gutter: 20.0,
        participant_gutter: 20.0,
        font_size: 10.0,
        padding: 5.0,
        corner_radius: 2.0,
    };

    // "Alice->Bob long name:Solving the system each time make for faster updates and allow to keep the solver in a consinstent state. However, the variable values are not updated automatically and you need to ask the solver to perform this operation before reading the values as illustrated below\nJohn->Bob long name:iiiiiiiiiiiiiiiiiiiii\nBob long name->John:It's Alice\nBob long name->Alice:I'm fine\n".to_string(),

    let src = r#"
        participant John
        Alice->+John: Hello John, how are you?
        Alice->+John: John, can you hear me?
        John->-Alice: Hi Alice, I can hear you!
        John->-Alice: I feel great!
    "#;

    match sequence_diagram::parser::parse(src.to_string()) {
        Ok(diagram) => {
            to_svg(&diagram, &svg_config);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
