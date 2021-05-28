use hirofa_utils::js_utils::{JsError, Script, ScriptPreProcessor};
use std::sync::Arc;

use swc::config::Config;
use swc_common::errors::{ColorConfig, Handler};
use swc_common::{FileName, SourceMap};
use swc_ecma_parser::{JscTarget, Syntax};

pub struct TypeScriptPreProcessor {}

impl TypeScriptPreProcessor {
    pub fn new() -> Self {
        Self {}
    }
    // todo custom target
    // todo keep instance of compiler in arc (lazy_static)
    pub fn transpile(&self, code: &str) -> Result<String, JsError> {
        let cm = Arc::<SourceMap>::default();
        let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
        let c = swc::Compiler::new(cm.clone(), Arc::new(handler));

        //let fm = cm
        //    .load_file(Path::new("foo.ts"))
        //    .expect("failed to load file");

        let fm = cm.new_source_file(FileName::Custom("test.ts".into()), code.into());

        let ts_cfg = swc_ecma_parser::TsConfig {
            dynamic_import: true,
            decorators: true,
            ..Default::default()
        };

        let mut cfg = Config::default();
        cfg.jsc.syntax = Some(Syntax::Typescript(ts_cfg));
        cfg.jsc.external_helpers = false;
        cfg.jsc.target = Some(JscTarget::Es2016);

        let ops = swc::config::Options {
            config: cfg,
            ..Default::default()
        };

        let res = c.process_js_file(fm, &ops);

        match res {
            Ok(to) => Ok(to.code),
            Err(e) => Err(JsError::new_string(format!("{}", e))),
        }
    }
}

impl Default for TypeScriptPreProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptPreProcessor for TypeScriptPreProcessor {
    fn process(&self, script: &mut Script) -> Result<(), JsError> {
        if script.get_path().ends_with(".ts") || script.get_path().ends_with(".mts") {
            // todo different options for modules?
            let js = self.transpile(script.get_code())?;
            script.set_code(js);
        }
        log::debug!(
            "TypeScriptPreProcessor:process file={} result = {}",
            script.get_path(),
            script.get_code()
        );

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::TypeScriptPreProcessor;
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_ts() {
        let rt = green_copper_runtime::new_greco_rt_builder()
            .script_pre_processor(TypeScriptPreProcessor::new())
            .build();

        let res = rt
            .eval_sync(Script::new(
                "test.ts",
                "(function(a: Number, b, c) {let d: String = 'abc'; return(a);}(1, 2, 3))",
            ))
            .ok()
            .expect("script failed");
        println!("res = {:?}", res);
        assert_eq!(res.get_i32(), 1);
    }
}
