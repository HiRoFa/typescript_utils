use hirofa_utils::js_utils::{JsError, Script, ScriptPreProcessor};
use std::sync::Arc;

use swc::config::{Config, ModuleConfig};
use swc::ecmascript::ast::EsVersion;
use swc_common::errors::{ColorConfig, Handler};
use swc_common::{FileName, SourceMap};
use swc_ecma_parser::{Syntax, TsConfig};
use ModuleConfig::Es6;

pub enum TargetVersion {
    Es3,
    Es5,
    Es2016,
    Es2020,
}

pub struct TypeScriptPreProcessor {
    minify: bool,
    external_helpers: bool,
    target: EsVersion,
}

impl TypeScriptPreProcessor {
    pub fn new(target: TargetVersion, minify: bool, external_helpers: bool) -> Self {
        let target = match target {
            TargetVersion::Es3 => EsVersion::Es3,
            TargetVersion::Es5 => EsVersion::Es5,
            TargetVersion::Es2016 => EsVersion::Es2016,
            TargetVersion::Es2020 => EsVersion::Es2020,
        };
        Self {
            minify,
            external_helpers,
            target,
        }
    }
    // todo custom target
    // todo keep instance of compiler in arc (lazy_static)
    pub fn transpile(&self, code: &str, is_module: bool) -> Result<String, JsError> {
        let cm = Arc::<SourceMap>::default();
        let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

        let c = swc::Compiler::new(cm.clone());

        //let fm = cm
        //    .load_file(Path::new("foo.ts"))
        //    .expect("failed to load file");

        let fm = cm.new_source_file(FileName::Custom("test.ts".into()), code.into());

        let ts_cfg = TsConfig {
            dynamic_import: true,
            decorators: true,
            ..Default::default()
        };

        let mut cfg = Config::default();
        cfg.jsc.syntax = Some(Syntax::Typescript(ts_cfg));
        cfg.jsc.target = Some(self.target);
        cfg.jsc.external_helpers = self.external_helpers;
        cfg.minify = self.minify;
        // todo setup sourcemaps for minify to work
        cfg.module = Some(Es6);

        let ops = swc::config::Options {
            config: cfg,
            is_module,
            ..Default::default()
        };

        let res = c.process_js_file(fm, &handler, &ops);

        match res {
            Ok(to) => Ok(to.code),
            Err(e) => Err(JsError::new_string(format!("{}", e))),
        }
    }
}

impl Default for TypeScriptPreProcessor {
    fn default() -> Self {
        Self::new(TargetVersion::Es2016, false, true)
    }
}

impl ScriptPreProcessor for TypeScriptPreProcessor {
    fn process(&self, script: &mut Script) -> Result<(), JsError> {
        if script.get_path().ends_with(".ts") {
            let js = self.transpile(script.get_code(), false)?;
            script.set_code(js);
        } else if script.get_path().ends_with(".mts") {
            let js = self.transpile(script.get_code(), true)?;
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
    use crate::TargetVersion;
    use crate::TypeScriptPreProcessor;
    use futures::executor::block_on;
    use hirofa_utils::js_utils::facades::{JsRuntimeBuilder, JsRuntimeFacade};
    use hirofa_utils::js_utils::{Script, ScriptPreProcessor};
    use quickjs_runtime::builder::QuickJsRuntimeBuilder;

    #[test]
    fn test_ts() {
        let rt = QuickJsRuntimeBuilder::new()
            .js_script_pre_processor(TypeScriptPreProcessor::new(
                TargetVersion::Es2020,
                false,
                false,
            ))
            .build();

        let fut = rt.js_eval(
            None,
            Script::new(
                "test.ts",
                "(function(a: Number, b, c) {let d: String = 'abc'; return(a);}(1, 2, 3))",
            ),
        );
        let res = block_on(fut).ok().expect("script failed");
        //println!("res = {}", res.js_get_type());
        assert_eq!(res.get_i32(), 1);
    }

    #[test]
    fn test_mts() {
        let pp = TypeScriptPreProcessor::new(TargetVersion::Es2020, true, true);
        let inputs = vec![
             Script::new(
                "import_test.mts",
                "import {a} from 'foo.mts'; \n{let b: Number = a.quibus;}\n export function q(val: Number) {};",
            ),
             Script::new(
                 "export_class_test.mts",
                 "export class MyClass {constructor(name) {this.name = name;}}",
             )
        ];

        for mut input in inputs {
            match pp.process(&mut input) {
                Ok(_) => {
                    assert!(!input.get_code().is_empty());
                    println!(
                        "{}\n-------------\n{}\n---------------\n",
                        input.get_path(),
                        input.get_code()
                    );
                }
                Err(err) => {
                    panic!("{}:\n-------------\n{}", input.get_path(), err);
                }
            }
        }
    }
}
