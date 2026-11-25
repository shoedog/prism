from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
bp = Blueprint("auth", __name__)


@bp.route("/<int:id>/u")
def account(id):
    return {"id": id}


app.register_blueprint(bp, url_prefix="/v1", name="auth_v1")
app.register_blueprint(bp, url_prefix="/v2", name="auth_v2")
