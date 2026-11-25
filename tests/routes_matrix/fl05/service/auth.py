from flask import Blueprint

bp = Blueprint("auth", __name__, url_prefix="/auth")


@bp.route("/<int:id>/u")
def account(id):
    return {"id": id}
