# typescript_utils
Typescript utils for GreCo

This enables you to transpile typescript to js for use with quickjs.

I placed these in a separate project because they require rust nightly.

# usage

Create a new rust project, add the following to the ```Cargo.toml```

```toml
[dependencies]
green_copper_runtime = {git = "https://github.com/HiRoFa/GreenCopperRuntime"}
typescript_utils = {git = "https://github.com/HiRoFa/typescript_utils"}
```

Add a ```rust-toolchain.toml``` file with the following content
```toml
[toolchain]
channel = "nightly"
```

Then you can create a runtime and run typescript using the following code
```rust
let rt = green_copper_runtime::new_greco_rt_builder()
    .script_pre_processor(TypeScriptPreProcessor::new())
    .build();

let res = rt.eval_sync(
    Script::new(
        "test.ts",
        "(function(a: Number, b, c) {let d: String = 'abc'; return(a);}(1, 2, 3))"
    )
).ok().expect("script failed");

println!("res = {:?}", res);
assert_eq!(res.get_i32(), 1);
```