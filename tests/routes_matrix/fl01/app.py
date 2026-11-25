from flask import Flask

app = Flask(__name__, static_folder=None)


@app.route("/items", methods=["GET", "POST"])
def items():
    return {"items": []}
