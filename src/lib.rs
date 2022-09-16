use hirofa_utils::js_utils::{JsError, Script, ScriptPreProcessor};
use std::sync::Arc;
use swc::{Compiler};

use swc_common::errors::{ColorConfig, Handler};
use swc_common::{FileName, SourceMap};

pub enum TargetVersion {
    Es3,
    Es5,
    Es2016,
    Es2020,
    Es2021,
    Es2022,
}

impl TargetVersion {
    fn as_str(&self) -> &str {
        match self {
            TargetVersion::Es3 => "es3",
            TargetVersion::Es5 => "es5",
            TargetVersion::Es2016 => "es2016",
            TargetVersion::Es2020 => "es2020",
            TargetVersion::Es2021 => "es2020",
            TargetVersion::Es2022 => "es2020"
        }
    }
}

pub struct TypeScriptPreProcessor {
    minify: bool,
    external_helpers: bool,
    target: TargetVersion,
    compiler: Compiler,
    source_map: Arc<SourceMap>,
}

impl TypeScriptPreProcessor {
    pub fn new(target: TargetVersion, minify: bool, external_helpers: bool) -> Self {

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
    pub fn transpile(&self, code: &str, file_name: &str, is_module: bool) -> Result<String, JsError> {
        let handler = Handler::with_tty_emitter(
            ColorConfig::Auto,
            true,
            false,
            Some(self.source_map.clone()),
        );

        let fm = self
            .source_map
            .new_source_file(FileName::Custom(file_name.into()), code.into());

        let minify_options = if self.minify {
            format!(r#"
                "minify": {{
                  "compress": {{
                    "unused": {}
                  }},
                  "mangle": true
                }},
            "#, is_module)
        } else {
            "".to_string()
        };

        let module = if is_module { r#"
                "module": {
                    "type": "es6",
                    "strict": true,
                    "strictMode": true,
                    "lazy": false,
                    "noInterop": false,
                    "ignoreDynamic": true
                },
                "#
        } else {
            ""
        };

        let cfg_json = format!(r#"

            {{
              "minify": {},
              {}
              "jsc": {{
                {}
                "externalHelpers": {},
                "parser": {{
                  "syntax": "typescript",
                  "tsx": true,
                  "decorators": true,
                  "dynamicImport": true
                }},
                "transform": {{
                  "legacyDecorator": true,
                  "decoratorMetadata": true,
                  "react": {{
                      "runtime": "classic",
                      "useBuiltins": true,
                      "refresh": true
                  }}
                }},
                "target": "{}"
              }}
            }}

        "#, self.minify, module, minify_options, self.external_helpers, self.target.as_str());

        log::trace!("using config {}", cfg_json);

        let cfg = serde_json::from_str(cfg_json.as_str()).map_err(|e| {
            JsError::new_string(format!("{}", e))
        })?;

        let ops = swc::config::Options {
            config: cfg,
            ..Default::default()
        };

        let res = self.compiler.process_js_file(fm, &handler, &ops);

        match res {
            Ok(to) => {
                Ok(to.code)
            },
            Err(e) => Err(JsError::new_string(format!("transpile failed: {}", e))),
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

            let js = self.transpile(code, script.get_path(), is_module)?;
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
    use hirofa_utils::js_utils::{ Script, ScriptPreProcessor};
    use log::LevelFilter;
    use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    use simple_logging::log_to_stderr;

    #[test]
    fn test_ts() {

        log_to_stderr(LevelFilter::Trace);

        let rt = QuickJsRuntimeBuilder::new()
            .js_script_pre_processor(TypeScriptPreProcessor::new(
                TargetVersion::Es2020,
                false,
                true,
            ))
            .build();

        let fut = rt.js_eval(
            None,
            Script::new(
                "test.ts",
                "(function(a: Number, b, c) {let d: String = 'abc'; return(a);}(1, 2, 3))",
            ),
        );
        let res = match block_on(fut) {
            Ok(r) => r,
            Err(e) => panic!("script failed{}", e)
        };
        //println!("res = {}", res.js_get_type());
        assert_eq!(res.get_i32(), 1);
    }

    #[test]
    fn test_mts() {

        log_to_stderr(LevelFilter::Trace);

        let pp = TypeScriptPreProcessor::new(TargetVersion::Es2020, true, true);
        let inputs = vec![
            Script::new(
                "export_class_test.ts",
                "function functWithLongName(abc) {return abc + 1;};export class /* hi */ MyClass {name: string; sum: number; constructor(name) {this.name = name; this.sum = functWithLongName(1);} getIt() {return (this.name + ' is gotten');}}",
            ),
             Script::new(
                "import_test.ts",
                "import {MyClass} from 'export_class_test.ts'; \n{let b: Number = MyClass.quibus;}\n export function q(val: Number) {let mc = new MyClass(); return mc.sum + mc.getIt();};",
            ),
             Script::new(
                 "not_a_module.ts",
                 "async function test() {let m = await import('export_class_test.ts'); let mc = new m.MyClass(); console.log(m.getIt());}",
             ),

             Script::new(
                 "ssr.tsx",
                 r#"
                    import React, { Component } from 'react';
                    import Button from './Button'; // Import a component from another file

                    class DangerButton extends Component {
                      render(): void {
                        return <Button color="red" />;
                      }
                      render2(): void {
                        return <div />;
                      }
                    }

                    export default DangerButton;
                 "#,
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
