from flask import Flask, request
app = Flask(__name__)

def lookup():
    value = request.args.get("mode")
    return value

@app.get("/preview")
def show():
    return lookup()
