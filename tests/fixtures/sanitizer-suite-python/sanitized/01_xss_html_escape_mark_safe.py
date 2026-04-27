from flask import Flask, request
from django.utils.safestring import mark_safe
import html

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    escaped = html.escape(name)
    return mark_safe(escaped)
