import flask

app = flask.Flask(__name__, static_folder=None)


@app.get("/status")
def status():
    return "ok"
