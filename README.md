# epsilon

Run the app in the simulator with:

```
bazel run //Sources/App
```

`rules_xcodeproj` is used to generate the Xcode project from Bazel BUILD files,
which serve as the declarative sources of truth. Generate the project with:

```
bazel run //:xcodeproj
```

## Testing RustLib

```
bazel test //Sources/RustLib:tests
```

To view the output of the test produced by Cargo add the argument
`--test_output=all`. To then also tell Cargo to show outputs from the tests,
pass in `--test_arg=--nocapture`. To rerun a test without caching use
`--cache_test_results=no`.

