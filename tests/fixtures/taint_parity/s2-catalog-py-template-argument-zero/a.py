from flask import render_template_string

def entry():
    t = acquire() # @SRC
    return render_template_string(t, label='fixed') # @SINK
