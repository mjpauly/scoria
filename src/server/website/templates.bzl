'''Adds on the template header and footer to an html file.'''
def html_template(name, src, visibility=None):
    native.genrule(
        name = name,
        srcs = [src, ":template_top", ":template_bottom"],
        outs = [name + ".html"],
        cmd = """
            cat $(location :template_top) >> $@
            cat $(location %s) >> $@
            cat $(location :template_bottom) >> $@
            """ % src,
        visibility = visibility,
    )

'''Converts the input markdown file to html using cmark and adds the template
header and footer.'''
def md_template(name, src, include_html=False, visibility=None):
    native.genrule(
        name = name,
        srcs = [src, ":template_top", ":template_bottom"],
        outs = [name + ".html"],
        cmd = """
            cmark $(location %s) -t html %s > content.html

            cat $(location :template_top) >> $@
            cat content.html >> $@
            cat $(location :template_bottom) >> $@
            """ % (
            src,
            ("--unsafe" if include_html else ""),
        ),
        visibility = visibility,
    )

'''Creates a collection of filegroup rules containing a single file in each
from a dictionary of {target name: src file} definitions.'''
def files(names_srcs, visibility=None):
    for name, src in names_srcs.items():
        native.filegroup(
            name = name,
            srcs = [src],
            visibility = visibility,
        )

