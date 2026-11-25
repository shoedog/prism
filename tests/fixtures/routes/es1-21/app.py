from flask import Blueprint, Flask

app = Flask(__name__, static_folder=None)
section = Blueprint("section", __name__, url_prefix="/section")

@section.get("")
@section.get("/")
def index():
    return "section"

app.register_blueprint(section)
