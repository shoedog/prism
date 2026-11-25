from flask import Blueprint

__all__ = ["blueprint"]
blueprint = Blueprint("current", __name__)

@blueprint.get("/current")
def current():
    return "current"
