from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None, subdomain_matching=True)
app.config["SERVER_NAME"] = "example.test"
portal = Blueprint("portal", __name__, url_prefix="/portal", subdomain="team")
audit = Blueprint("audit", __name__, url_prefix="/audit", subdomain="archive")

@audit.get("/activity")
def activity():
    return "activity"

portal.register_blueprint(audit, subdomain="events")
app.register_blueprint(portal)
