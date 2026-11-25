from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
reports = Blueprint("reports", __name__)


@reports.get("/x")
def status():
    return "ok"


app.register_blueprint(reports, url_prefix="/a" + "/")
