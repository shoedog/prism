from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
portal = Blueprint("portal", __name__, url_prefix="/p")
catalog = Blueprint("catalog", __name__, url_prefix="/c")


@catalog.get("/x")
def catalog_status():
    return "ok"


portal.register_blueprint(catalog)
app.register_blueprint(portal)
