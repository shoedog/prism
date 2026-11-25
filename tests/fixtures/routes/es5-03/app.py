from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
audit = Blueprint("audit", __name__)


def archive():
    return {"archived": True}


audit.add_url_rule(view_func=archive, methods=["POST"], rule="/archive")
app.register_blueprint(url_prefix="/ops", blueprint=audit)
