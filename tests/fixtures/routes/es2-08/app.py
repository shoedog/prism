from flask import Flask

app = Flask(__name__, static_folder=None)

def options_report():
    return {"available": True}

options_report.required_methods = {"POST"}
app.add_url_rule("/report", view_func=options_report, methods=["GET"], provide_automatic_options=False)
