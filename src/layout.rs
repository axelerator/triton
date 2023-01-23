#![allow(unused)] // FIXME
                  //
use itertools::Itertools;
use std::borrow::Cow;
use std::collections::HashMap;

use cassowary::strength::{MEDIUM, REQUIRED, STRONG, WEAK};
use cassowary::WeightedRelation::*;
use cassowary::{Constraint, Expression, Solver, Variable};

use rusttype::{point, Font, Scale};

use textwrap::{wrap, LineEnding, Options};

#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub enum AlignmentAnchor {
    Start,
    Middle,
    End,
}

pub type Alignment = (Orientation, AlignmentAnchor);

pub struct Layout<'a> {
    solver: Solver,
    vars: HashMap<Variable, VariableId>,
    blocks: Vec<Block>,
    constraints_accu: Vec<Constraint>,
    right_var: Variable,
    bottom_var: Variable,
    right_: f64,
    bottom_: f64,
    font: Font<'a>,
    pub glyphs_height: f64,
}

impl Layout<'_> {
    pub fn new() -> Layout<'static> {
        let right_var = Variable::new();
        let bottom_var = Variable::new();
        let mut vars = HashMap::new();
        vars.insert(right_var, VariableId::LayoutRight);
        vars.insert(bottom_var, VariableId::LayoutBottom);

        // Read the font data.
        let font = include_bytes!("../resources/fonts/Roboto-Regular.ttf") as &[u8];
        // Parse it into the font type.
        let roboto_regular = Font::try_from_bytes(font).expect("Error constructing Font");

        let scale = Scale::uniform(12.0);
        let v_metrics = roboto_regular.v_metrics(scale);
        let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as f64;

        Layout {
            solver: Solver::new(),
            vars,
            blocks: vec![],
            constraints_accu: vec![],
            right_var,
            bottom_var,
            right_: 0.0,
            bottom_: 0.0,
            font: roboto_regular,
            glyphs_height,
        }
    }

    pub fn add_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        let block = Block::new(self, id);
        self.constraints_accu
            .push(self.right_var | GE(REQUIRED) | block.right());
        self.constraints_accu
            .push(self.bottom_var | GE(REQUIRED) | block.bottom());
        self.blocks.push(block);
        id
    }

    pub fn add_text_block(
        &mut self,
        content: &String,
        max_length: usize,
        padding: f64,
    ) -> (BlockId, Vec<String>) {
        let id = self.blocks.len();
        let mut block = Block::new(self, id);
        block.line_height = self.glyphs_height;
        self.constraints_accu
            .push(self.right_var | GE(REQUIRED) | block.right());
        self.constraints_accu
            .push(self.bottom_var | GE(REQUIRED) | block.bottom());

        let mut text_width = 0;

        let lines = wrap(content.as_str(), max_length);
        // The font size to use
        let scale = Scale::uniform(12.0);
        let v_metrics = self.font.v_metrics(scale);

        let mut height = 2.0 * padding;
        for line in &lines {
            // layout the glyphs in a line with 20 pixels padding
            let glyphs: Vec<_> = self
                .font
                .layout(line, scale, point(0.0, 0.0 + v_metrics.ascent))
                .collect();
            // work out the layout size
            height += self.glyphs_height as f64;
            let glyphs_width = {
                let min_x = glyphs
                    .first()
                    .map(|g| g.pixel_bounding_box().unwrap().min.x)
                    .unwrap();
                let max_x = glyphs
                    .last()
                    .map(|g| g.pixel_bounding_box().unwrap().max.x)
                    .unwrap();
                let line_width = (max_x - min_x) as u32;
                if line_width > text_width {
                    text_width = line_width;
                }
                line_width
            };
        }

        let width: f64 = (text_width as f64) + (2.0 * padding);
        self.constraints_accu
            .push(block.width | GE(REQUIRED) | width);
        self.constraints_accu.push(block.width | EQ(WEAK) | width);

        self.constraints_accu
            .push(block.height | GE(STRONG) | (height as f64));

        self.blocks.push(block);
        let liness: Vec<String> = lines
            .as_slice()
            .iter()
            .map(|c| c.clone().into_owned())
            .collect();
        (id, liness)
    }

    pub fn b(&self, id: BlockId) -> &Block {
        &self.blocks[id]
    }

    pub fn add_var(&mut self, block_id: BlockId, block_var: BlockVariable) -> Variable {
        let var = Variable::new();
        self.vars
            .insert(var, VariableId::BlockVar(block_id, block_var));
        var
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints_accu.push(constraint);
    }

    pub fn solve(&mut self) {
        self.solver.add_constraints(&self.constraints_accu).unwrap();
        for &(var, value) in self.solver.fetch_changes() {
            let var_id = self.vars[&var];
            let value = if value.is_sign_negative() { 0.0 } else { value };
            match var_id {
                VariableId::LayoutRight => self.right_ = value,
                VariableId::LayoutBottom => self.bottom_ = value,
                VariableId::BlockVar(block_id, attr) => {
                    let mut block = self.blocks.get_mut(block_id).unwrap();

                    match attr {
                        BlockVariable::X => {
                            block.x_ = value;
                        }
                        BlockVariable::Y => {
                            block.y_ = value;
                        }
                        BlockVariable::Width => {
                            block.width_ = value;
                        }
                        BlockVariable::Height => {
                            block.height_ = value;
                        }
                    }
                }
            }
        }
        self.constraints_accu = vec![];
    }

    pub fn width(&self) -> f64 {
        self.right_
    }

    pub fn height(&self) -> f64 {
        self.bottom_
    }

    pub fn distribute<'a>(
        &mut self,
        orientation: Orientation,
        gutter: f64,
        block_ids: impl Iterator<Item = &'a BlockId>,
    ) {
        match orientation {
            Orientation::Vertical => {
                for (prev, next) in block_ids.tuple_windows() {
                    let prev_block = self.b(*prev);
                    let next_block = self.b(*next);
                    self.add_constraint(
                        prev_block.bottom() + gutter | LE(REQUIRED) | next_block.top(),
                    );
                }
            }
            Orientation::Horizontal => {
                for (prev, next) in block_ids.tuple_windows() {
                    let prev_block = self.b(*prev);
                    let next_block = self.b(*next);
                    self.add_constraint(
                        prev_block.right() + gutter | LE(REQUIRED) | next_block.left(),
                    );
                }
            }
        }
    }
    pub fn align<'a>(
        &mut self,
        orientation: Orientation,
        anchor: AlignmentAnchor,
        block_ids: impl Iterator<Item = &'a BlockId>,
    ) {
        match orientation {
            Orientation::Vertical => {
                for (prev, next) in block_ids.tuple_windows() {
                    let prev_block = self.b(*prev);
                    let next_block = self.b(*next);

                    match anchor {
                        AlignmentAnchor::Start => {
                            self.add_constraint(prev_block.top() | EQ(REQUIRED) | next_block.top());
                        }
                        AlignmentAnchor::Middle => {
                            self.add_constraint(
                                prev_block.top() + (prev_block.height * 0.5)
                                    | EQ(REQUIRED)
                                    | next_block.top() + (next_block.height * 0.5),
                            );
                        }
                        AlignmentAnchor::End => {
                            self.add_constraint(
                                prev_block.bottom() | EQ(REQUIRED) | next_block.bottom(),
                            );
                        }
                    }
                }
            }
            Orientation::Horizontal => {
                for (prev, next) in block_ids.tuple_windows() {
                    let prev_block = self.b(*prev);
                    let next_block = self.b(*next);

                    match anchor {
                        AlignmentAnchor::Start => {
                            self.add_constraint(
                                prev_block.left() | EQ(REQUIRED) | next_block.left(),
                            );
                        }
                        AlignmentAnchor::Middle => {
                            self.add_constraint(
                                prev_block.left() + (prev_block.width * 0.5)
                                    | EQ(REQUIRED)
                                    | next_block.left() + (next_block.width * 0.5),
                            );
                        }
                        AlignmentAnchor::End => {
                            self.add_constraint(
                                prev_block.right() | EQ(REQUIRED) | next_block.right(),
                            );
                        }
                    }
                }
            }
        }
    }
}

pub type BlockId = usize;

#[derive(Debug, Clone, Copy)]
enum VariableId {
    BlockVar(BlockId, BlockVariable),
    LayoutRight,
    LayoutBottom,
}

#[derive(Debug, Clone, Copy)]
pub enum BlockVariable {
    X,
    Y,
    Width,
    Height,
}

pub struct Block {
    x: Variable,
    y: Variable,
    pub width: Variable,
    pub height: Variable,

    pub x_: f64,
    pub y_: f64,
    pub width_: f64,
    pub height_: f64,
    pub line_height: f64,
}

impl Block {
    pub fn new(layout: &mut Layout, id: BlockId) -> Block {
        let x = layout.add_var(id, BlockVariable::X);
        let y = layout.add_var(id, BlockVariable::Y);
        let width = layout.add_var(id, BlockVariable::Width);
        let height = layout.add_var(id, BlockVariable::Height);

        layout.constraints_accu.push(x | GE(REQUIRED) | 0f64);
        layout.constraints_accu.push(y | GE(REQUIRED) | 0f64);
        layout.constraints_accu.push(width | GE(REQUIRED) | 0f64);
        layout.constraints_accu.push(height | GE(REQUIRED) | 0f64);

        Block {
            x,
            y,
            width,
            height,
            x_: 0.0,
            y_: 0.0,
            width_: 0.0,
            height_: 0.0,
            line_height: 0.0,
        }
    }

    pub fn left(&self) -> Variable {
        self.x
    }

    pub fn top(&self) -> Variable {
        self.y
    }

    pub fn right(&self) -> Expression {
        self.x + self.width
    }

    pub fn bottom(&self) -> Expression {
        self.y + self.height
    }
}
