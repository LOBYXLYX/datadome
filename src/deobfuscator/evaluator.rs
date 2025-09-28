use rusty_v8 as v8;

#[derive(Debug)]
pub struct JsRuntime {
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
}

impl JsRuntime {
    pub fn new() -> Self {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let mut isolate = v8::Isolate::new(Default::default());

        let context = {
            let handle_scope = &mut v8::HandleScope::new(&mut isolate);
            let context = v8::Context::new(handle_scope);

            v8::Global::new(handle_scope, context)
        };

        JsRuntime { isolate, context }
    }

    pub fn eval(&mut self, code: &str) -> Option<String> {
        let handle_scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(handle_scope, &self.context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let src = v8::String::new(scope, code).unwrap();
        let mut try_catch = v8::TryCatch::new(scope);

        let script = v8::Script::compile(&mut try_catch, src, None);

        if let Some(script) = script {
            if let Some(result) = script.run(&mut try_catch) {
                let result_str = result.to_string(&mut try_catch).unwrap();
                return Some(result_str.to_rust_string_lossy(&mut try_catch));
            }
        }

        if let Some(exception) = try_catch.exception() {
            let exception_str = exception
                .to_string(&mut try_catch)
                .unwrap()
                .to_rust_string_lossy(&mut try_catch);
            return Some(format!("Error: {}", exception_str));
        }

        None
    }
}
