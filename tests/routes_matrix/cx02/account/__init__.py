from flask import Blueprint

bp = Blueprint("account", __name__, url_prefix="/account")


@bp.get("/summary")
def summary():
    return {"active": True}
