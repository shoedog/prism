from flask import Flask
from account import bp

app = Flask(__name__, static_folder=None)
app.register_blueprint(bp)
