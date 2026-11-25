from flask import render_template_string
def entry():
    t = acquire() # @SRC
    return render_template_string(
        "{{ name | safe }}",
        other="fixed",
        name=t # @SINK
    )
