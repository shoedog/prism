from flask import render_template_string
def f(inp):
    t = inp.value # @SRC
    render_template_string("{{ name | safe }}", other="fixed", name=t) # @SINK
