from flask import Flask, request
import os
app = Flask(__name__)
@app.get("/edition")
def edition():
    fallback = os.environ["DEFAULT_EDITION"]
    if "title" in request.args:
        return ""
    title = request.args.get("title", fallback)
    return title
