use hirofa_utils::js_utils::{JsError, Script, ScriptPreProcessor};
use std::sync::Arc;
use swc::Compiler;

use swc::config::util::BoolOrObject;
use swc::config::{Config, IsModule, JsMinifyOptions, ModuleConfig, TerserSourceMapOption};
use swc::ecmascript::ast::EsVersion;
use swc_common::errors::{ColorConfig, Handler};
use swc_common::{FileName, SourceMap};
use swc_ecma_minifier::option::{MangleOptions, ManglePropertiesOptions};
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
    compiler: Compiler,
    source_map: Arc<SourceMap>,
}

impl TypeScriptPreProcessor {
    pub fn new(target: TargetVersion, minify: bool, external_helpers: bool) -> Self {
        let target = match target {
            TargetVersion::Es3 => EsVersion::Es3,
            TargetVersion::Es5 => EsVersion::Es5,
            TargetVersion::Es2016 => EsVersion::Es2016,
            TargetVersion::Es2020 => EsVersion::Es2020,
        };
        let source_map = Arc::<SourceMap>::default();
        let compiler = swc::Compiler::new(source_map.clone());

        Self {
            minify,
            external_helpers,
            target,
            source_map,
            compiler,
        }
    }
    // todo custom target
    pub fn transpile(&self, code: &str, is_module: bool) -> Result<String, JsError> {
        let handler = Handler::with_tty_emitter(
            ColorConfig::Auto,
            true,
            false,
            Some(self.source_map.clone()),
        );

        let fm = self
            .source_map
            .new_source_file(FileName::Custom("test.ts".into()), code.into());

        let ts_cfg = TsConfig {
            decorators: true,
            ..Default::default()
        };

        let mut cfg = Config::default();
        cfg.jsc.syntax = Some(Syntax::Typescript(ts_cfg));
        cfg.jsc.target = Some(self.target);
        cfg.jsc.external_helpers = self.external_helpers;
        if self.minify {
            cfg.minify = true;
            cfg.jsc.minify = Some(JsMinifyOptions {
                compress: Default::default(),
                mangle: BoolOrObject::Obj(MangleOptions {
                    props: Some(ManglePropertiesOptions {
                        reserved: vec![],
                        undeclared: false,
                        regex: None,
                    }),
                    top_level: true,
                    keep_class_names: false,
                    keep_fn_names: false,
                    keep_private_props: false,
                    ie8: false,
                    safari10: false,
                }),
                format: Default::default(),
                ecma: Default::default(),
                keep_classnames: false,
                keep_fnames: false,
                module: is_module,
                safari10: false,
                toplevel: true,
                source_map: BoolOrObject::Obj(TerserSourceMapOption {
                    filename: None,
                    url: None,
                    root: None,
                    content: None,
                }),
                output_path: None,
                inline_sources_content: false,
            });
        }

        // todo setup sourcemaps for minify to work
        cfg.module = Some(Es6);

        let ops = swc::config::Options {
            config: cfg,
            is_module: IsModule::Bool(is_module),
            ..Default::default()
        };

        let res = self.compiler.process_js_file(fm, &handler, &ops);

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
            let code = script.get_code();

            let is_module = code.starts_with("import ")
                || code.starts_with("export ")
                || code.contains(" import ")
                || code.contains("\nimport ")
                || code.contains("\timport ")
                || code.contains(";import ")
                || code.contains(" export ")
                || code.contains("\nexport ")
                || code.contains("\texport ")
                || code.contains(";export ");

            let js = self.transpile(code, is_module)?;
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
                "import_test.ts",
                "import {a} from 'foo.ts'; \n{let b: Number = a.quibus;}\n export function q(val: Number) {};",
            ),
             Script::new(
                 "export_class_test.ts",
                 "function functWithLongName(abc) {return abc + 1;};export class /* hi */ MyClass {constructor(name) {this.name = name; this.sum = functWithLongName(1);} getIt() {return (this.name + ' is gotten');}}",
             ),
             Script::new(
                 "not_a_module.ts",
                 "async function test() {let m = await import('testmod.ts');}",
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
