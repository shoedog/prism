from flask import Flask

app = Flask(__name__, static_folder=None)
app.config["DEBUG"] = True

if app.config["DEBUG"]:
    @app.get("/diagnostics")
    def diagnostics():
        return "ok"
