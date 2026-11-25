from flask import Flask

app = Flask(__name__, static_folder=None)
M = ["GET", "POST"]


@app.route("/items", methods=M)
def items():
    return {"items": []}
