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
