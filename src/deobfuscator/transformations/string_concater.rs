use oxc_allocator::{Box as AstBox, Allocator};
use oxc_ast::ast::*;
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::walk_expression;
use crate::string_lit_alloc;

trait Evaluable {
    fn eval(&self) -> Option<Value>;
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Number(i64),
    Str(String),
}
pub struct StringConcatenator<'a> {
    pub allocator: &'a Allocator
}

impl<'a> Evaluable for Expression<'a> {
    fn eval(&self) -> Option<Value> {
        match self {
            Expression::StringLiteral(str_lit) => {
                Some(Value::Str(str_lit.value.to_string()))
            }
            Expression::NumericLiteral(num) => {
                Some(Value::Number(num.value as i64))
            }
            Expression::BinaryExpression(bin) => bin.eval(),
            _ => None,
        }
    }
}

impl<'a> Evaluable for BinaryExpression<'a> {
    fn eval(&self) -> Option<Value> {
        let left: Value = self.left.eval()?;
        let right: Value = self.right.eval()?;

        match self.operator {
            BinaryOperator::Addition => {
                match (left, right) {
                    (Value::Number(l), Value::Number(r)) => Some(Value::Number(l + r)),
                    (Value::Str(l), Value::Str(r)) => Some(Value::Str(l + &r)),
                    (Value::Number(l), Value::Str(r)) => Some(Value::Str(format!("{}{}", l, r))),
                    (Value::Str(l), Value::Number(r)) => Some(Value::Str(format!("{}{}", l, r))),
                }
            }
            _ => None,
        }
    }
}

impl<'a> VisitMut<'a> for StringConcatenator<'a> {
    fn visit_expression(&mut self, expr: &mut Expression<'a>) {
        match expr {
            Expression::BinaryExpression(bin) => {
                self.visit_expression(&mut bin.left);
                self.visit_expression(&mut bin.right);

                match (bin.left.eval(), bin.right.eval()) {
                    (Some(Value::Str(l)), Some(Value::Str(r))) => {
                        let combined: String = format!("{}{}", l, r);

                        *expr = string_lit_alloc!(combined.as_str(), self, bin);
                    }
                    _ => {}
                }
            }
            _ => walk_expression(self, expr),
        }
    }
}

