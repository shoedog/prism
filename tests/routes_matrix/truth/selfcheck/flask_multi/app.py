from flask import Flask
from parts.blueprint import bp


def create_app():
    app = Flask(__name__, static_folder=None)
    app.register_blueprint(bp, name="original")
    app.register_blueprint(bp, url_prefix="/second", name="another")
    return app
