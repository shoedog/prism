import json
from flask import Flask, request

app = Flask(__name__)

@app.post("/edition")
def edition():
    wire = request.form["entry"]
    actual = json.loads(wire)
    if not isinstance(actual, dict):
        return ""
    masked = json.loads(wire, object_hook=lambda obj: "preview")
    return [actual, masked]
