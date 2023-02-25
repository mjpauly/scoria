# generate our wasm with trunk (ideally this would be done with rules_rust)
# install trunk with $ cargo install --locked trunk

genrule(
    name = "front_wasm",
    srcs = [
        "index.html",
        "styles/input.css",
        "rs/main.rs",
        "Cargo.toml",
        "Cargo.lock",
        "tailwind.config.js",
    ],
    cmd = """
mkdir rs
mkdir styles
cp $(location index.html) .
cp $(location styles/input.css) styles
cp $(location rs/main.rs) rs
cp $(location Cargo.toml) .
cp $(location Cargo.lock) .
cp $(location tailwind.config.js) .
npx tailwindcss -i ./styles/input.css -o ./styles/output.css
trunk build
zip -r $@ dist
    """,
    outs = ["dist.zip"],
    visibility = ["//visibility:public"],
)
