from flask import Flask, request
from django.utils.html import format_html

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    return format_html("<b>{}</b>", name)
