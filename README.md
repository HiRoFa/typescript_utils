# typescript_utils
Typescript utils for GreCo

This enables you to transpile typescript to js for use with quickjs.

# usage

Create a new rust project, add the following to the ```Cargo.toml```

```toml
[dependencies]
quickjs_runtime = {git = "https://github.com/HiRoFa/quickjs_es_runtime"}
typescript_utils = {git = "https://github.com/HiRoFa/typescript_utils"}
futures = "0.3.6"
```

Then you can create a runtime and run typescript using the following code
```rust
use crate::TypeScriptPreProcessor;
use futures::executor::block_on;
use hirofa_utils::js_utils::facades::{JsRuntimeBuilder, JsRuntimeFacade};
use hirofa_utils::js_utils::Script;
use quickjs_runtime::builder::QuickJsRuntimeBuilder;

#[test]
fn test_ts() {
    let rt = QuickJsRuntimeBuilder::new()
        .js_script_pre_processor(TypeScriptPreProcessor::new())
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
```