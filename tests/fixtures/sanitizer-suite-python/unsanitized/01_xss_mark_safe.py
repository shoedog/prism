from flask import Flask, request
from django.utils.safestring import mark_safe

app = Flask(__name__)

@app.route("/profile")
def profile():
    name = request.args.get("name")
    return mark_safe(name)
