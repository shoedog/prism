from flask import Flask

app = Flask(__name__, static_folder=None)


def status():
    return "ok"


def authorize(view):
    def guarded():
        return view()

    return guarded


app.add_url_rule("/status", methods=["GET"], view_func=authorize(status))
