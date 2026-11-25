from flask import Flask, request
app = Flask(__name__)

@app.get("/search")
def search():
    term = request.args.get("term")
    return term
