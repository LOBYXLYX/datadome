use oxc_ast::ast::{SwitchStatement, TryStatement, FunctionBody, BlockStatement};

#[derive(Debug)]
pub enum NodeTypes<'a> {
    Switch(SwitchStatement<'a>),
    FuncBody(FunctionBody<'a>),
    Block(BlockStatement<'a>)
}

#[derive(Debug)]
pub struct ParentHandler<'a> {
    stack: Vec<NodeTypes<'a>>
}

impl<'a> ParentHandler<'a> {
    pub fn new() -> Self {
        Self {
            stack: Vec::new()
        }
    }

    pub fn push(&mut self, node: NodeTypes<'a>) {
        self.stack.push(node);
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub fn last(&mut self) -> Option<&NodeTypes> {
        self.stack.last()
    }

    pub fn nodes(&mut self) -> Vec<NodeTypes<'a>> {
        let mut nodes: Vec<NodeTypes<'a>> = Vec::new();
        while let Some(node) = self.stack.pop() {
            nodes.push(node);
        };
        nodes
    }
}