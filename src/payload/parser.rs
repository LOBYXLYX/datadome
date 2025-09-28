use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, VisitMut};
use oxc_allocator::CloneIn;
use oxc_allocator::{Box as AstBox, Allocator};
use oxc_semantic::ScopeFlags;
use oxc_ast_visit::walk::{
    walk_variable_declarator, 
    walk_assignment_expression, 
    walk_block_statement, 
    walk_call_expression, 
    walk_for_statement,
    walk_function_body, 
    walk_if_statement, 
    walk_switch_statement, 
    walk_try_statement
};
use oxc_ast_visit::walk_mut;
use crate::deobfuscator::transformations::utils::{
    generate_func_code, 
    generate_var_code, 
    generate_call_code,
    generate_assignment_expr_code,
    generate_block_statement_code,
    generate_function_body_code,
    generate_switch_statement_code
};
use crate::payload::utils::{NodeTypes, ParentHandler};

const ONG_FR: [&'static str; 7] = ["denied", "oldBrowser", "noGpu", "noAd", "nosup", "NA", "noIframe"];

struct EncoderParser<'a> {
    dynamic_num: i64,
    functions: Vec<(String, String)>, // (name, code)
    allocator: &'a Allocator,
    program: Program<'a>
}

pub enum KeyType { Str, Identifier, Number }
#[derive(Debug)]
pub enum NodeVarType<'a> { 
    AssignExpr(AssignmentExpression<'a>), 
    VarDecl(VariableDeclarator<'a>) 
}

struct KeyData {
    key_type: KeyType,
    constant_var: Option<String>,
    block: Option<String>,
    expression: Option<String>
}

struct KeysParser<'a> {
    functions: &'a Vec<(String, String)>,
    found_keys: Vec<String>,
    parsed_keys: Vec<KeyData>,
    parents: ParentHandler<'a>,
    allocator: &'a Allocator,
    program: Program<'a>
}

struct TraverseParents<'a> {
    identifier: String,
    found_node: bool,
    allocator: &'a Allocator,
    node: Option<NodeVarType<'a>>
}

impl<'a> TraverseParents<'a> {
    pub fn new(identifier: String, allocator: &'a Allocator) -> Self {
        Self {
            identifier: identifier, 
            found_node: false,
            allocator: allocator,
            node: None
        }
    }
}

impl<'a> Visit<'a> for TraverseParents<'a> {
    fn visit_assignment_expression(&mut self, assign_expr: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(assign_ident) = &assign_expr.left {
            if assign_ident.name.as_str() == self.identifier {
                self.found_node = true;
                self.node = Some(NodeVarType::AssignExpr(assign_expr.clone_in(self.allocator)));
            }
        }
        walk_assignment_expression(self, assign_expr);
    }

    fn visit_variable_declarator(&mut self, var_decl: &VariableDeclarator<'a>) {
        if let Some(ident) = &var_decl.id.get_binding_identifier() {
            if ident.name.as_str() == self.identifier {
                self.found_node = true;
                self.node = Some(NodeVarType::VarDecl(var_decl.clone_in(self.allocator)));
            }
        }
        walk_variable_declarator(self, var_decl);
    }
}

impl<'a> KeysParser<'a> {
    pub fn new(functions: &'a Vec<(String, String)>, allocator: &'a Allocator, program: Program<'a>) -> Self {
        Self {
            functions,
            found_keys: Vec::new(),
            parsed_keys: Vec::new(),
            parents: ParentHandler::new(),
            allocator,
            program
        }
    }
}

impl<'a> VisitMut<'a> for EncoderParser<'a> {
    fn visit_object_expression(&mut self, objexpr: &mut ObjectExpression<'a>) {
        if objexpr.properties.len() == 1 {
            let property = &objexpr.properties[0];

            if let ObjectPropertyKind::ObjectProperty(prop) = property {
                if let PropertyKey::StaticIdentifier(key) = &prop.key {
                    if key.name.len() == 1 {
                        if let Expression::UnaryExpression(unary) = &prop.value {
                            if let Expression::NumericLiteral(n) = &unary.argument {
                                let dyn_num: Option<i64> = match unary.operator {
                                    UnaryOperator::UnaryNegation => Some(-(n.value as i64)),
                                    _ => None
                                };

                                self.dynamic_num = dyn_num.unwrap();
                            }
                        } else if let Expression::NumericLiteral(n) = &prop.value {
                            self.dynamic_num = n.value as i64;
                        }
                    }
                }
            }
        }

        walk_mut::walk_object_expression(self, objexpr);
    }

    fn visit_function(&mut self, _func: &mut Function<'a>, flags: ScopeFlags) {
        if let Some(ident) = &_func.id {
            let func_code = generate_func_code(self.allocator, &mut self.program, _func);
            self.functions.push((ident.name.to_string(), func_code));
        }

        walk_mut::walk_function(self, _func, flags);
    }

    fn visit_variable_declarator(&mut self, var_decl: &mut VariableDeclarator<'a>) { 
        if let Some(ident) = var_decl.id.get_binding_identifier() {
            if let Some(_init) = &var_decl.init {
                if let Expression::FunctionExpression(_) = _init {
                    let func_expr_code = generate_var_code(self.allocator, &mut self.program, ident.name.as_str(), var_decl.init.take());

                    self.functions.push((ident.name.to_string(), func_expr_code));
                }
            }
        }
        walk_mut::walk_variable_declarator(self, var_decl);
    }
}

impl<'a> KeysParser<'a> {
    fn find_function(&self, func_name: String) -> Option<&String> {
        let mut function = None;
        for (name, code) in self.functions {
            if *name == func_name {
                function = Some(code);
                break;
            }
        }
        function
    }

    fn generate_parent_code(&mut self, node: NodeTypes<'a>) -> String {
        match node {
            NodeTypes::FuncBody(f) => generate_function_body_code(self.allocator,  &f),
            NodeTypes::Switch(s) => generate_switch_statement_code(self.allocator, &mut self.program, s.discriminant.clone_in(self.allocator), &s.cases),
            NodeTypes::Block(b) => generate_block_statement_code(self.allocator, &b)
        }
    }

    fn parse_call_expr(&mut self, call_expr: &AstBox<'a, CallExpression<'a>>) {
        let args = &call_expr.arguments;

        if args.len() == 1 {
            println!("cdwqdq {:?}", args);
            if let Argument::BinaryExpression(b) = &args[0] {
                if let Expression::StringLiteral(s) = &b.left {
                    if s.value.contains("Err: ") {
                        return;
                    }
                }
            } else if let Argument::Identifier(ident) = &args[0] {
                let mut traverse: TraverseParents = TraverseParents::new(ident.name.to_string(), self.allocator);
                let nodes: Vec<NodeTypes<'a>> = self.parents.nodes();
                let mut expected_node: Option<NodeTypes<'a>> = None;

                for nodet in nodes {
                    match nodet {
                        NodeTypes::Block(ref b) => traverse.visit_block_statement(&b),
                        NodeTypes::FuncBody(ref f) => traverse.visit_function_body(&f),
                        NodeTypes::Switch(ref s) => traverse.visit_switch_statement(&s),
                    }

                    if traverse.found_node {
                        expected_node = Some(nodet);
                        break;
                    }
                }

                if let Some(_node) = &traverse.node {
                    if let NodeVarType::AssignExpr(assign) = _node {
                        if let Expression::CallExpression(call) = &assign.right {
                            todo!()
                        } else {
                            if let AssignmentTarget::AssignmentTargetIdentifier(assign_ident) = &assign.left {
                                println!("{:#?} {:#?}", call_expr, assign);
                                let block = Some(self.generate_parent_code(expected_node.unwrap()));
                                self.parsed_keys.push(KeyData {
                                    key_type: KeyType::Identifier,
                                    constant_var: Some(generate_assignment_expr_code(self.allocator, &mut self.program, assign_ident.name.as_str(), &assign.right)),
                                    block: block,
                                    expression: Some(generate_call_code(self.allocator, &mut self.program, call_expr))
                                })
                            }   
                        }
                    } else if let NodeVarType::VarDecl(var) = _node {
                        if let Some(_init) = &var.init {
                            if let Expression::CallExpression(call) = _init {
                                todo!()
                            } else {
                                println!("var {:#?}", _node);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Visit<'a> for KeysParser<'a> {
    fn visit_call_expression(&mut self, _call: &CallExpression<'a>) {
        if let Expression::Identifier(ident) = &_call.callee {
            let fn_name = ident.name.to_string();
            if _call.arguments.len() == 2 {
                let args = &_call.arguments;

                if let Argument::StringLiteral(s) = &args[0] {
                    if s.value.len() == 6 {
                        let key = s.value.to_string();

                        if self.found_keys.contains(&key) {
                            walk_call_expression(self, _call);
                        };
                        
                        //if let Argument::Identifier(id) = &args[1] {
                        //    let identifier_name = id.name.to_string();
                        //    

                        //}

                        if let Argument::StringLiteral(arg_s) = &args[1] {
                            if arg_s.value.contains("Err:") {
                                walk_call_expression(self, _call);
                            };

                            if ONG_FR.contains(&arg_s.value.as_str()) {
                                return;
                            }

                            self.found_keys.push(key.clone());
                            self.parsed_keys.push(KeyData {
                                key_type: KeyType::Str,
                                block: Some(arg_s.value.to_string()),
                                constant_var: None,
                                expression: None
                            })
                        } else if let Argument::CallExpression(arg_c) = &args[1] {
                            if let Expression::Identifier(ident) = &arg_c.callee {
                                let func_code = self.find_function(ident.name.to_string());

                                self.found_keys.push(key.clone());
                                self.parse_call_expr(arg_c);
                            }
                            walk_call_expression(self, arg_c);
                        }
                    }
                }
            }
        }
        walk_call_expression(self, _call);
    }

    fn visit_switch_statement(&mut self, stmt: &SwitchStatement<'a>) {
        self.parents.push(NodeTypes::Switch(stmt.clone_in(self.allocator)));
        walk_switch_statement(self, stmt);
        self.parents.pop();
    }

    fn visit_block_statement(&mut self, stmt: &BlockStatement<'a>) {
        self.parents.push(NodeTypes::Block(stmt.clone_in(self.allocator)));
        walk_block_statement(self, stmt);
        self.parents.pop();
    }

    fn visit_function_body(&mut self, func_body: &FunctionBody<'a>) {
        self.parents.push(NodeTypes::FuncBody(func_body.clone_in(self.allocator)));
        walk_function_body(self, func_body);
        self.parents.pop(); 
    }
} 

pub fn dynamic_parser<'a>(mut program: Program<'a>, allocator: &'a Allocator) -> (i64, Vec<(String, String)>, Program<'a>) {
    let mut enc_visitor = EncoderParser { 
        dynamic_num: 0 , 
        functions: Vec::new(),
        allocator: allocator,
        program: program.clone_in(allocator)
    };

    enc_visitor.visit_program(&mut program);

    (enc_visitor.dynamic_num, enc_visitor.functions, program)
}

pub fn parse_payload_keys<'a>(program: &Program, functions: &Vec<(String, String)>, allocator: &'a Allocator) {
    let mut keys_parser = KeysParser::new(functions, allocator, program.clone_in(allocator));

    keys_parser.visit_program(program);
}