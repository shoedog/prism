from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
billing = Blueprint("billing", __name__, url_prefix="/billing")

@billing.get("/history")
def history():
    return "history"

app.register_blueprint(billing, name="default_billing", url_prefix=None)
app.register_blueprint(billing, name="root_billing", url_prefix="")
