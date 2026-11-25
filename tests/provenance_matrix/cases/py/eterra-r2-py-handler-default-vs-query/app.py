from flask import Flask, request
app = Flask(__name__)
@app.get("/")
def handler():
    term, fallback = request.args.get("q"), "fixed"
    return term, fallback
