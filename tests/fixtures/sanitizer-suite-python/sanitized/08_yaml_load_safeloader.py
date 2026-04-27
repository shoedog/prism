from flask import Flask, request
import yaml

app = Flask(__name__)

@app.route("/load")
def load_config():
    payload = request.get_data()
    return yaml.load(payload, Loader=yaml.SafeLoader)
