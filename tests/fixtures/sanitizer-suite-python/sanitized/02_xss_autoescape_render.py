from flask import Flask, request, render_template_string

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    return render_template_string("Hello {{ name }}", name=name)
