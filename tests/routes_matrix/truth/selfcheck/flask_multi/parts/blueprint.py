from flask import Blueprint

bp = Blueprint("items", __name__, url_prefix="/v1")


@bp.get("/one")
@bp.post("/two")
def handler():
    return ""


bp.add_url_rule("/three", view_func=handler, methods=["put", "put"]); bp.add_url_rule("/four", view_func=handler, methods=["PATCH"])
