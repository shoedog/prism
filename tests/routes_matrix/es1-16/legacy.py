from flask import Blueprint

__all__ = ["blueprint"]
blueprint = Blueprint("legacy", __name__)

@blueprint.get("/legacy")
def legacy():
    return "legacy"
