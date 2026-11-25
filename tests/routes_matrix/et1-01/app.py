from flask import Flask
app = Flask(__name__, static_folder=None)
def report():
    return "ok"
app.add_url_rule("/report", view_func=report, methods=("POST", "GET", "GET"))
