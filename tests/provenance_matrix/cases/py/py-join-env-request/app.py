import os
from flask import Flask, request
app = Flask(__name__)

@app.get("/preview")
def show():
    if request.args.get("remote"):
        value = request.args.get("mode")
    else:
        value = os.getenv("MODE")
    return value
