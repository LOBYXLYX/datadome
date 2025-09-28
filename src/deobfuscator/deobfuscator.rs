
use std::fs;
use std::io::prelude::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast_visit::VisitMut;
use oxc_allocator::CloneIn;
use oxc_codegen::Codegen;
use oxc_ast::ast::Program;
use oxc_allocator::Allocator;
use crate::deobfuscator::transformations;
use crate::deobfuscator::evaluator::JsRuntime;

pub fn deobfuscate<'a>(source: &'a str, allocator: &'a Allocator) -> Program<'a> {
    let mut runtime: JsRuntime = JsRuntime::new();

    let parser = Parser::new(
        allocator,
        source,
        SourceType::default(),
    );
    let mut ret = parser.parse();

    let evaluator: &mut JsRuntime = &mut runtime;

    println!("step 1");
    
    let (char_coder_name, parsed_funcs_name, decoder_arrmapfn_name) = {
        let mut string_collector = transformations::string_decoder::StringEncoderCollector::new(
            allocator,
            ret.program.clone_in(allocator),
            evaluator
        );
        string_collector.visit_program(&mut ret.program);
        (
            string_collector.char_coder_name,
            string_collector.parsed_funcs_name,
            string_collector.decoder_arrmapfn_name,
        )
    };


    println!("{:#?}", parsed_funcs_name);
    println!("{:?}", decoder_arrmapfn_name);
    println!("{:?}", char_coder_name);

    let mut string_char_decoder = transformations::string_decoder::StringCharDecoder {
        allocator,
        char_coder_name,
        evaluator,
        program: ret.program.clone_in(allocator)
    };
    println!("step 2");
    string_char_decoder.visit_program(&mut ret.program);

    let mut string_decoder_1 = transformations::string_decoder::StringDecoder {
        allocator,
        parsed_funcs_name: parsed_funcs_name,
        evaluator,
        program: ret.program.clone_in(allocator)
    };
    println!("step 3");
    string_decoder_1.visit_program(&mut ret.program);
    let parsed_funcs_name = string_decoder_1.parsed_funcs_name;

    let mut string_concater = transformations::string_concater::StringConcatenator {
        allocator
    };

    println!("step 4");
    string_concater.visit_program(&mut ret.program);

    let mut array_string: Vec<String> = Vec::new();
    let mut array_collector = transformations::string_decoder::ArrayStringCollector {
        allocator,
        array_string: &mut array_string,
        evaluator,
        program: ret.program.clone_in(allocator)
    };
    println!("step 5");
    array_collector.visit_program(&mut ret.program);

    println!("{:#?} {}", parsed_funcs_name, decoder_arrmapfn_name);

    let mut decoder_final = transformations::string_decoder::StringDecoderFinal {
        allocator,
        parsed_funcs_name: parsed_funcs_name,
        decoder_arrmapfn_name,
        array_strings: array_string,
        evaluator,
        program: ret.program.clone_in(allocator)
    };
    println!("step 6");
    decoder_final.visit_program(&mut ret.program);

    if let Ok(mut r) = fs::File::create("src/assets/output/result_3.js") {
        let result: String = Codegen::new().build(&ret.program).code;
        let _ = r.write_all(&result.clone().into_bytes());
    };

    ret.program
}
