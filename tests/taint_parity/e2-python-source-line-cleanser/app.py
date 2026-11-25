from flask import Flask, request, render_template_string
app = Flask(__name__)
@app.route("/")
def f():
    escape("fixed")
    return render_template_string("{{ name | safe }}", name=request.args.get("name")) # @SINK

def parity_anchor():
    t = acquire() # @SRC
