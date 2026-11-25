from flask import Blueprint, Flask

catalog = Blueprint("catalog", __name__)

@catalog.get("/catalog/<int:item>")
def show(item):
    return str(item)

app = Flask(__name__)
app.register_blueprint(catalog)
