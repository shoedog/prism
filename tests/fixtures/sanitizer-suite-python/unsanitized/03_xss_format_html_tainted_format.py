from flask import Flask, request
from django.utils.html import format_html

app = Flask(__name__)

@app.route("/profile")
def profile():
    fmt = request.args.get("fmt")
    return format_html(fmt, "value")
