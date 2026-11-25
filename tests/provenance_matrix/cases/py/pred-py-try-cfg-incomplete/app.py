from flask import Flask, request
app = Flask(__name__)

@app.get("/search")
def show():
    term = ""
    try:
        term = request.args.get("term")
    finally:
        print(term)
