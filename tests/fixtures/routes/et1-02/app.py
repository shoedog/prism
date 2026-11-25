from flask import Flask
app = Flask(__name__, static_folder=None, host_matching=True)
@app.route("/items/", host="api.example.test", strict_slashes=False, methods=["GET"])
def items():
    return "ok"
