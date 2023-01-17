#![allow(unused)] // FIXME
                  //
use itertools::Itertools;
use std::collections::HashMap;

use cassowary::strength::{MEDIUM, REQUIRED, STRONG, WEAK};
use cassowary::WeightedRelation::*;
use cassowary::{Constraint, Expression, Solver, Variable};

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

pub struct Layout {
    solver: Solver,
    vars: HashMap<Variable, VariableId>,
    blocks: Vec<Block>,
    constraints_accu: Vec<Constraint>,
    right_var: Variable,
    bottom_var: Variable,
    right_: f64,
    bottom_: f64,
}

impl Layout {
    pub fn new() -> Layout {
        let right_var = Variable::new();
        let bottom_var = Variable::new();
        let mut vars = HashMap::new();
        vars.insert(right_var, VariableId::LayoutRight);
        vars.insert(bottom_var, VariableId::LayoutBottom);
        Layout {
            solver: Solver::new(),
            vars,
            blocks: vec![],
            constraints_accu: vec![],
            right_var,
            bottom_var,
            right_: 0.0,
            bottom_: 0.0,
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
