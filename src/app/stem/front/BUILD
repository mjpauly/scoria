# generate our wasm with trunk (ideally this would be done with rules_rust)
# install trunk with $ cargo install --locked trunk

genrule(
    name = "front_wasm",
    srcs = [
        "index.html",
        "styles/input.css",
        "Cargo.toml",
        "Cargo.lock",
        "tailwind.config.js",
        "rs/main.rs",
        "rs/lib.rs",
        "rs/sense.rs",
        "rs/analyze.rs",
        "rs/test_page.rs",
        "rs/common.rs",
    ],
    cmd = """
mkdir rs
mkdir styles
cp $(location index.html) .
cp $(location styles/input.css) styles
cp $(location Cargo.toml) .
cp $(location Cargo.lock) .
cp $(location tailwind.config.js) .
cp $(location rs/main.rs) rs
cp $(location rs/lib.rs) rs
cp $(location rs/sense.rs) rs
cp $(location rs/analyze.rs) rs
cp $(location rs/test_page.rs) rs
cp $(location rs/common.rs) rs

npx tailwindcss -i ./styles/input.css -o ./styles/output.css
trunk build
zip -r $@ dist
    """,
    outs = ["dist.zip"],
    visibility = ["//visibility:public"],
)
