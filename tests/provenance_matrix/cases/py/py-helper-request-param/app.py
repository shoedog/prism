from flask import Flask, request
app = Flask(__name__)

def lookup(req):
    value = req.args.get("mode")
    return value

@app.get("/preview")
def show():
    return lookup(request)
