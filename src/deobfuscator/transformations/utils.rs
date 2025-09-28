use oxc_codegen::Codegen;
use oxc_allocator::CloneIn;
use oxc_allocator::{Allocator, Box as AstBox, Vec as AstVec};
use oxc_ast::ast::*;
use oxc_span::{Atom, SPAN};


pub fn generate_func_code_mut<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    func: &mut Function<'a>
) -> String {
    let boxed_func = AstBox::new_in(func.clone_in(allocator), allocator);

    let mut stmts: AstVec<'a, Statement<'a>> = AstVec::new_in(allocator);
    stmts.push(Statement::FunctionDeclaration(boxed_func));
    program.body = stmts;
    Codegen::new().build(&program).code
}

pub fn generate_call_code_mut<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    call: &mut AstBox<'a, CallExpression<'a>>
) -> String {
    let mut stmts: AstVec<'a, Statement<'a>> = AstVec::new_in(allocator);
    let expr_stmt = ExpressionStatement {
        expression: Expression::CallExpression(call.clone_in(allocator)),
        span: call.span,
    };

    stmts.push(Statement::ExpressionStatement(AstBox::new_in(expr_stmt, allocator)));
    program.body = stmts;

    Codegen::new().build(&program).code
}

pub fn generate_call_code<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    call: &AstBox<'a, CallExpression<'a>>
) -> String {
    let mut stmts: AstVec<'a, Statement<'a>> = AstVec::new_in(allocator);
    let expr_stmt = ExpressionStatement {
        expression: Expression::CallExpression(call.clone_in(allocator)),
        span: call.span,
    };

    stmts.push(Statement::ExpressionStatement(AstBox::new_in(expr_stmt, allocator)));
    program.body = stmts;

    Codegen::new().build(&program).code
}

pub fn generate_func_code<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    func: &Function<'a>
) -> String {
    let boxed_func = AstBox::new_in(func.clone_in(allocator), allocator);

    let mut stmts: AstVec<'a, Statement<'a>> = AstVec::new_in(allocator);
    stmts.push(Statement::FunctionDeclaration(boxed_func));
    program.body = stmts;
    Codegen::new().build(&program).code
}

pub fn generate_var_code<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    name: &'a str,
    init: Option<Expression<'_>>,
) -> String {
    let ident = allocator.alloc(BindingIdentifier {
        name: Atom::from(name),
        span: SPAN,
        symbol_id: Default::default(),
    });

    let _init = if let Some(i) = init {
        i
    } else {
        panic!("fwe");
    };

    let binding_pattern = BindingPattern {
        kind: BindingPatternKind::BindingIdentifier(AstBox::new_in((*ident).clone_in(allocator), allocator)),
        type_annotation: None,
        optional: false, 
    };

    let declarator = allocator.alloc(VariableDeclarator {
        span: SPAN,
        kind: VariableDeclarationKind::Var,
        id: binding_pattern,
        init: Some(_init),
        definite: false,
    });

    let mut decls: AstVec<'a, VariableDeclarator<'a>> = AstVec::new_in(allocator);
    decls.push(declarator.clone_in(allocator));

    let var_decl = allocator.alloc(VariableDeclaration {
        span: SPAN,
        kind: VariableDeclarationKind::Var, // 🔑 siempre "var"
        declarations: decls,
        declare: false,
    });

    let mut stmts = AstVec::new_in(allocator);
    stmts.push(Declaration::VariableDeclaration(AstBox::new_in((*var_decl).clone_in(allocator), allocator)).into());

    program.body = stmts;

    Codegen::new().build(&program).code
}

pub fn generate_assignment_expr_code<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    left_name: &'a str,
    right: &Expression<'a>,
) -> String {
    let ident = allocator.alloc(IdentifierReference {
        name: Atom::from(left_name),
        span: SPAN,
        reference_id: Default::default(),
    });

    let target = AssignmentTarget::AssignmentTargetIdentifier(
        AstBox::new_in((*ident).clone_in(allocator), allocator),
    );

    let assign_expr = allocator.alloc(AssignmentExpression {
        span: SPAN,
        operator: AssignmentOperator::Assign,
        left: target,
        right: right.clone_in(allocator), 
    });

    let expr_stmt = allocator.alloc(Statement::ExpressionStatement(
        AstBox::new_in(ExpressionStatement {
            span: SPAN,
            expression: Expression::AssignmentExpression(
                AstBox::new_in((*assign_expr).clone_in(allocator), allocator),
            )
        }, allocator)
    ));

    let mut stmts = AstVec::new_in(allocator);
    stmts.push((*expr_stmt).clone_in(allocator));
    program.body = stmts;

    Codegen::new().build(&program).code
}

pub fn generate_switch_statement_code<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    discriminant: Expression<'a>,
    cases: &AstVec<'a, SwitchCase<'a>>,
) -> String {
    let switch_stmt = AstBox::new_in(
        SwitchStatement {
            span: SPAN,
            discriminant,
            cases: cases.clone_in(allocator),
            scope_id: None.into(),
        },
        allocator,
    );

    let mut stmts = AstVec::new_in(allocator);
    stmts.push(Statement::SwitchStatement(switch_stmt));
    program.body = stmts;
    Codegen::new().build(&program).code
}
pub fn generate_block_statement_code<'a>(
    allocator: &'a Allocator,
    block: &BlockStatement<'a>,
) -> String {
    let mut code = String::from("{\n");

    for stmt in &block.body {
        let stmt_code = generate_statement_code(allocator, stmt);
        code.push_str(&stmt_code);
        code.push('\n');
    }

    code.push('}');
    code
}

fn generate_statement_code<'a>(
    allocator: &'a Allocator,
    stmt: &Statement<'a>,
) -> String {
    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            generate_expression_code(allocator, &expr_stmt.expression) + ";"
        }
        Statement::ReturnStatement(ret_stmt) => {
            if let Some(arg) = &ret_stmt.argument {
                format!("return {};", generate_expression_code(allocator, arg))
            } else {
                "return;".to_string()
            }
        }
        Statement::BlockStatement(block) => generate_block_statement_code(allocator, block),
        _ => String::from("/* unsupported statement */"),
    }
}

// Expression generator dummy
fn generate_expression_code<'a>(
    _allocator: &'a Allocator,
    _expr: &Expression<'a>,
) -> String {
    "/* expr */".to_string()
}

pub fn generate_function_body_code<'a>(
    allocator: &'a Allocator,
    body: &FunctionBody<'a>,
) -> String {
    let mut code = String::from("{\n");

    for directive in &body.directives {
        code.push_str(&format!("\"{}\";\n", directive.directive.as_str()));
    }

    for stmt in &body.statements {
        let stmt_code = generate_statement_code(allocator, stmt);
        code.push_str(&stmt_code);
        code.push('\n');
    }

    code.push('}');
    code
}


pub fn fromcharcode(char_code: u32) -> String {
    let c = std::char::from_u32(char_code).unwrap();
    c.to_string()
}