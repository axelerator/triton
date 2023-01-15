#![allow(unused)] // FIXME
                  //
use std::collections::HashMap;

use cassowary::strength::{MEDIUM, REQUIRED, STRONG, WEAK};
use cassowary::WeightedRelation::*;
use cassowary::{Constraint, Expression, Solver, Variable};

pub struct Layout {
    solver: Solver,
    vars: HashMap<Variable, VariableId>,
    blocks: Vec<Block>,
    constraints_accu: Vec<Constraint>,
}

impl Layout {
    pub fn new() -> Layout {
        Layout {
            solver: Solver::new(),
            vars: HashMap::new(),
            blocks: vec![],
            constraints_accu: vec![],
        }
    }

    pub fn add_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        let block = Block::new(self, id);
        self.blocks.push(block);
        id
    }

    pub fn b(&self, id: BlockId) -> &Block {
        &self.blocks[id]
    }

    pub fn add_var(&mut self, block_id: BlockId, block_var: BlockVariable) -> Variable {
        let var = Variable::new();
        self.vars.insert(var, (block_id, block_var));
        var
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints_accu.push(constraint);
    }

    pub fn solve(&mut self) {
        println!("Solving {:?} constraints.", self.constraints_accu.len());
        self.solver.add_constraints(&self.constraints_accu).unwrap();
        for &(var, value) in self.solver.fetch_changes() {
            println!("{:?}: {:?}", var, value);
            let (block_id, attr) = self.vars[&var];
            let value = if value.is_sign_negative() { 0.0 } else { value };

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
        self.constraints_accu = vec![];
    }
}

pub type BlockId = usize;
type VariableId = (BlockId, BlockVariable);

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
