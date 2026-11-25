from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
bp = Blueprint("auth", __name__, url_prefix="/auth")


@bp.route("/<int:id>/u")
def account(id):
    return {"id": id}


app.register_blueprint(bp, url_prefix="/v1")
