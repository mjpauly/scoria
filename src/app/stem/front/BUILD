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
        "rs/router.rs",
        "rs/common.rs",

        "rs/pages/mod.rs",
        "rs/pages/sense.rs",
        "rs/pages/analyze.rs",
        "rs/pages/test_page.rs",

        "rs/components/mod.rs",
        "rs/components/navbar.rs",
    ],
    cmd = """
mkdir -p rs/pages
mkdir -p rs/components
mkdir styles

cp $(location index.html) .
cp $(location styles/input.css) styles
cp $(location Cargo.toml) .
cp $(location Cargo.lock) .
cp $(location tailwind.config.js) .

cp $(location rs/main.rs) rs
cp $(location rs/lib.rs) rs
cp $(location rs/router.rs) rs
cp $(location rs/common.rs) rs

cp $(location rs/pages/mod.rs) rs/pages
cp $(location rs/pages/sense.rs) rs/pages
cp $(location rs/pages/analyze.rs) rs/pages
cp $(location rs/pages/test_page.rs) rs/pages

cp $(location rs/components/mod.rs) rs/components
cp $(location rs/components/navbar.rs) rs/components

npx tailwindcss -i ./styles/input.css -o ./styles/output.css
trunk build
zip -r $@ dist
    """,
    outs = ["dist.zip"],
    visibility = ["//visibility:public"],
)
