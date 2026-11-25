from flask import Blueprint, Flask
bp = Blueprint("reports", __name__, url_prefix="/reports")
@bp.get("/daily")
def daily():
    return "ok"
def create_app():
    app = Flask(__name__, static_folder=None)
    app.register_blueprint(bp)
    return app
