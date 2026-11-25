from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
reports = Blueprint("reports", __name__)

def snapshot():
    return "snapshot"

@reports.record_once
def install_snapshot(state):
    state.add_url_rule("/snapshot", view_func=snapshot, methods=["GET"])

@reports.get("/current")
def current():
    return "current"

app.register_blueprint(reports, url_prefix="/first")
app.register_blueprint(reports, url_prefix="/second", name="reports_again")
