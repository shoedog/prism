from flask import Flask

from audit import blueprint

app = Flask(__name__, static_folder=None)
app.register_blueprint(blueprint, url_prefix="/audit")
