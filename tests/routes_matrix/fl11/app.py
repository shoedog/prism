from flask import Flask

app = Flask(__name__, static_folder=None)


def report():
    return {"status": "ready"}


app.add_url_rule("/r", view_func=report)
