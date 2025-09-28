use base64::Engine;
use oxc_allocator::{Box as AstBox, Allocator};
use oxc_ast::ast::*;
use oxc_ast_visit::VisitMut;
use oxc_ast_visit::walk_mut::{walk_call_expression, walk_expression, walk_function, walk_variable_declarator};
use crate::deobfuscator::evaluator::JsRuntime;
use crate::deobfuscator::transformations::utils;

enum EvaluatedValue {
    Number(f64),
    Str(String),
}

fn parse_value(value: &String) -> EvaluatedValue {
    if let Ok(n) = value.parse::<f64>() {
        EvaluatedValue::Number(n)
    } else {
        EvaluatedValue::Str(value.to_string())
    }
}

#[macro_export]
macro_rules! string_lit_alloc {
    (&$s: expr, &$t:expr, &$b:expr ) => {
        string_lit_alloc!(&s, &t, &b)
    };
    ($s:expr, $t:expr, $b:expr) => {
        Expression::StringLiteral(AstBox::new_in(
            StringLiteral {
                span: $b.span,
                value: Atom::from(&*$t.allocator.alloc_str($s)),
                raw: None,
                lone_surrogates: false,
            },
            $t.allocator,
        ))
    };
}

pub struct StringEncoderCollector<'a, 'eval> {
    pub allocator: &'a Allocator,
    pub parsed_funcs_name: Vec<String>,
    pub program: Program<'a>,
    pub char_coder_name: &'a str,
    pub decoder_arrmapfn_name: &'a str,
    pub evaluator: &'eval mut JsRuntime
}
// parcial decoder of string.fromcharcode or smth
// putting everything in a single impl can be slower
pub struct StringCharDecoder<'a, 'eval> {
    pub allocator: &'a Allocator,
    pub char_coder_name: &'a str,
    pub evaluator: &'eval mut JsRuntime,
    pub program: Program<'a>,
}

// decode litte stuff like tr(131) or ew(-214, 242)
pub struct StringDecoder<'a, 'eval> {
    pub allocator: &'a Allocator,
    pub parsed_funcs_name: Vec<String>,
    pub evaluator: &'eval mut JsRuntime,
    pub program: Program<'a>,
}


pub struct ArrayStringCollector<'a, 'eval> {
    pub allocator: &'a Allocator,
    pub array_string: &'eval mut Vec<String>,
    pub evaluator: &'eval mut JsRuntime,
    pub program: Program<'a>,
}

pub struct StringDecoderFinal<'a, 'eval> {
    pub allocator: &'a Allocator,
    pub parsed_funcs_name: Vec<String>,
    pub decoder_arrmapfn_name: &'a str,
    pub array_strings: Vec<String>,
    pub evaluator: &'eval mut JsRuntime,
    pub program: Program<'a>,
}

impl<'a, 'eval> StringDecoderFinal<'a, 'eval> {
    fn decode_arr_map_string(&self, index: usize) -> Option<String>{
        let mut b64_string = self.array_strings[index].clone();

        b64_string.retain(|c| !c.is_whitespace());

        let rem: usize = b64_string.len() % 4;
        if rem == 2 {
            b64_string.push_str("==");
        } else if rem == 3 {
            b64_string.push('=');
        }

        match base64::engine::general_purpose::STANDARD.decode(&b64_string) {
            Ok(decoded) => {
                let decoded_str: String = String::from_utf8_lossy(&decoded).to_string();
                return Some(decoded_str);
            }
            Err(e) => {
                println!("error to decode {} {:?}", b64_string, e);
                return None;
            }
        }
    }
}

impl<'a, 'eval> StringEncoderCollector<'a, 'eval> {
    pub fn new(allocator: &'a Allocator, program: Program<'a>, evaluator: &'eval mut JsRuntime) -> Self {
        Self {
            allocator,
            parsed_funcs_name: Vec::new(),
            program,
            char_coder_name: "",
            decoder_arrmapfn_name: "",
            evaluator
        }
    }
}

impl<'a, 'eval> VisitMut<'a> for StringEncoderCollector<'a, 'eval> {
    fn visit_variable_declarator(&mut self, var_decl: &mut VariableDeclarator<'a>) {
        if let Some(_init) = &mut var_decl.init {
            self.visit_expression(_init);
            if let Expression::StaticMemberExpression(_sm) = &_init {
                if _sm.property.name == "fromCharCode" {
                    let var_name = match var_decl.id.get_binding_identifier() {
                        Some(ident) => ident.name.as_str(),
                        _ => ""
                    };
                    self.char_coder_name = var_name;
                }
            }
        };
    }
    fn visit_function(&mut self, _func: &mut Function<'a>, _flags: oxc_semantic::ScopeFlags) {
        if let Some(id) = _func.id.clone() {
            if let Some(body) = &_func.body {
                if let Statement::ReturnStatement(_ret) = &body.statements[0] {
                    if body.statements.len() == 1 {
                        let func_code: String = utils::generate_func_code_mut(self.allocator, &mut self.program, _func);
                        
                        if func_code.len() < 170 && !func_code.contains("atob") {
                            self.evaluator.eval(func_code.as_str());
                            self.parsed_funcs_name.push(id.name.to_string());
                        } else if func_code.contains("atob") && func_code.len() < 120 {
                            self.decoder_arrmapfn_name = id.name.as_str();
                            self.parsed_funcs_name.push(id.name.to_string());
                        }
                    }
                } else if let Some(exprr) = body.statements.get(1) {
                    if let Statement::ReturnStatement(ret2) = exprr {
                        if let Some(arg_ret) = &ret2.argument {
                            if let Expression::BinaryExpression(_) = *arg_ret {
                                let func_code = utils::generate_func_code_mut(self.allocator, &mut self.program, _func);
                                self.evaluator.eval(func_code.as_str());
                                self.parsed_funcs_name.push(id.name.to_string());
                            } else {
                                let func_code = utils::generate_func_code_mut(self.allocator, &mut self.program, _func);
                                if func_code.contains("atob") && func_code.len() < 120 {
                                    self.decoder_arrmapfn_name = id.name.as_str();
                                    self.parsed_funcs_name.push(id.name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        walk_function(self, _func, _flags);
    }
}

impl<'a, 'eval> VisitMut<'a> for StringCharDecoder<'a, 'eval> {
    fn visit_expression(&mut self, expr: &mut Expression<'a>) {
        if let Expression::CallExpression(call) = expr {
            if let Expression::Identifier(ident) = &call.callee {
                if ident.name.as_str() == self.char_coder_name && call.arguments.len() == 1 {
                    if let Argument::NumericLiteral(num) = &call.arguments[0] {
                        let char: String = utils::fromcharcode(num.value as u32);

                        //println!("{:?}", string_lit);
                        *expr = string_lit_alloc!(char.as_str(), self, call);
                        return;
                    } else if let Argument::CallExpression(_call2) = &mut call.arguments[0] {
                        let value: Option<String> = self.evaluator.eval(&utils::generate_call_code_mut(self.allocator, &mut self.program, _call2));

                        match value {
                            Some(num) => {
                                let char: String = utils::fromcharcode(num.parse::<u32>().unwrap());
                                *expr = string_lit_alloc!(char.as_str(), self, call);
                                return;
                            },
                            _ => return
                        };
                    }
                }
            }
            walk_call_expression(self, call);
        }
    walk_expression(self, expr);
    }
}

impl<'a, 'eval> VisitMut<'a> for StringDecoder<'a, 'eval> {
    fn visit_function(&mut self, _func: &mut Function<'a>, _flags: oxc_semantic::ScopeFlags) {
        if let Some(id) = _func.id.clone() {
            if let Some(body) = &_func.body {
                if let Statement::ReturnStatement(_ret) = &body.statements[0] {
                    let func_code: String = utils::generate_func_code_mut(self.allocator, &mut self.program, _func);
                    
                    if func_code.len() > 500 && func_code.len() < 1600 && func_code.contains("fromCharCode") {
                        println!("{}", func_code);
                        self.evaluator.eval(func_code.as_str());
                        self.parsed_funcs_name.push(id.name.to_string());
                    }
                } else if let Some(_retstate) = body.statements.get(1) {
                    if let Statement::ReturnStatement(_ret) = &body.statements[1] {
                        let func_code: String = utils::generate_func_code_mut(self.allocator, &mut self.program, _func);
                    
                        if func_code.len() > 500 && func_code.len() < 1600 && func_code.contains("fromCharCode") {
                            println!("{}", func_code);
                            self.evaluator.eval(func_code.as_str());
                            self.parsed_funcs_name.push(id.name.to_string());
                        }
                    }
                }
            }
        };
        walk_function(self, _func, _flags);
    }
}


impl<'a, 'eval> VisitMut<'a> for ArrayStringCollector<'a, 'eval> {
    fn visit_variable_declarator(&mut self, var_decl: &mut VariableDeclarator<'a>) {
        if let Some(_init) = &mut var_decl.init {
            self.visit_expression(_init);

            if let Expression::ArrayExpression(arr) = &_init {
                let elements = &arr.elements;
                let var_name = match var_decl.id.get_binding_identifier() {
                    Some(ident) => ident.name.as_str(),
                    _ => ""
                };

                if elements.len() > 300 && elements.len() < 600 { // get only the base64 array strings
                    *self.array_string = elements.iter()
                        .map(|e| {
                            if let ArrayExpressionElement::StringLiteral(s) = e {
                                s.value.to_string()
                            } else if let ArrayExpressionElement::BinaryExpression(b) = e {
                                let left = &b.left;
                                let right = &b.right;
                                let concated = match (left, right) {
                                    (Expression::StringLiteral(l), Expression::ParenthesizedExpression(r)) => {
                                        if let Expression::StringLiteral(_r) = &r.expression {
                                            format!("{}{}", l.value, _r.value)
                                        } else {
                                            String::new()
                                        }
                                    },
                                    _ => String::new()
                                };
                                concated
                            } else {
                                String::new()
                            }
                        })
                        .collect::<Vec<String>>();
                } else if elements.len() > 100 {
                    let var_code = &utils::generate_var_code(
                        self.allocator,
                        &mut self.program,
                        var_name,
                        var_decl.init.take()
                    );
                    self.evaluator.eval(var_code);
                }
            }
        }
        walk_variable_declarator(self, var_decl);
    }
}

impl<'a, 'eval> VisitMut<'a> for StringDecoderFinal<'a, 'eval> {
    fn visit_expression(&mut self, expr: &mut Expression<'a>) {
        if let Expression::CallExpression(call) = expr {
            if let Expression::Identifier(ident) = &call.callee {
                if self.parsed_funcs_name.contains(&ident.name.to_string()) {
                    let args = &call.arguments;

                    if ident.name.as_str() != self.decoder_arrmapfn_name {
                        let value: Option<String> = self.evaluator.eval(&utils::generate_call_code_mut(self.allocator, &mut self.program, call));

                        if let Some(_v) = value {
                            if let EvaluatedValue::Str(v) = parse_value(&_v) {
                                *expr = string_lit_alloc!(v.as_str(), self, call);
                                return;
                            } else if let EvaluatedValue::Number(v) = parse_value(&_v) {
                                *expr = Expression::NumericLiteral(AstBox::new_in(
                                    NumericLiteral {
                                        span: call.span,
                                        value: v,
                                        raw: None,
                                        base: NumberBase::Binary
                                    },
                                    self.allocator
                                ));
                                return;
                            };
                        } else {
                            walk_call_expression(self, call);
                        }
                    } else if ident.name.as_str() == self.decoder_arrmapfn_name {
                        if let Argument::NumericLiteral(arg_num) = &args[0] {
                            let value: Option<String> = self.decode_arr_map_string(arg_num.value as usize);

                            if let Some(string) = value {
                                *expr = string_lit_alloc!(string.as_str(), self, call);
                                return;
                            } else {
                                return;
                            }
                        }
                    }
                }
            };
            walk_call_expression(self, call);
        }
    walk_expression(self, expr);
    }
}