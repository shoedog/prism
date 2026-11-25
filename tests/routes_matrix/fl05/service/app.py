from flask import Flask

from . import auth

app = Flask(__name__, static_folder=None)
app.register_blueprint(auth.bp)
