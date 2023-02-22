# generate our wasm with trunk (ideally this would be done with rules_rust)
# install trunk with $ cargo install --locked trunk

genrule(
    name = "front_wasm",
    srcs = [
        "index.html",
        "index.scss",
        "main.rs",
        "Cargo.toml",
        "Cargo.lock",
    ],
    cmd = """
cp $(location index.html) .
cp $(location index.scss) .
cp $(location main.rs) .
cp $(location Cargo.toml) .
cp $(location Cargo.lock) .
trunk build
zip -r $@ dist
    """,
    outs = ["dist.zip"],
    visibility = ["//visibility:public"],
)
