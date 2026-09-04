"""Builds targets for linux/amd64 regardless of the command-line platform.

Applied to a dependency subgraph rather than the whole build, so `bazel run`
targets like oci_push resolve their own tools for the host while the image
they operate on is cross-compiled.
"""

def _linux_amd64_transition_impl(settings, attr):
    return {
        "//command_line_option:platforms": ["@zig_sdk//platform:linux_amd64"],
        "//command_line_option:extra_toolchains": settings["//command_line_option:extra_toolchains"] + [
            "@zig_sdk//toolchain:linux_amd64_gnu.2.34",
        ],
    }

_linux_amd64_transition = transition(
    implementation = _linux_amd64_transition_impl,
    inputs = ["//command_line_option:extra_toolchains"],
    outputs = [
        "//command_line_option:platforms",
        "//command_line_option:extra_toolchains",
    ],
)

def _linux_amd64_files_impl(ctx):
    return [DefaultInfo(files = depset(transitive = [
        src[DefaultInfo].files
        for src in ctx.attr.srcs
    ]))]

linux_amd64_files = rule(
    implementation = _linux_amd64_files_impl,
    attrs = {
        "srcs": attr.label_list(cfg = _linux_amd64_transition),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
    doc = "Forwards the files of srcs, built for linux/amd64.",
)
