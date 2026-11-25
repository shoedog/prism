from flask import Flask

app = Flask(__name__, static_folder=None)


@app.route("/x")
def status():
    return {"status": "ok"}
