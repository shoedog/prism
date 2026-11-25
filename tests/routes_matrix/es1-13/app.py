from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
east = Blueprint("east", __name__, url_prefix="/east")
west = Blueprint("west", __name__, url_prefix="/west")
reports = Blueprint("reports", __name__, url_prefix="/reports")

@reports.get("/daily")
def daily():
    return "daily"

east.register_blueprint(reports)
west.register_blueprint(reports, url_prefix="/archive")
app.register_blueprint(east)
app.register_blueprint(west)
