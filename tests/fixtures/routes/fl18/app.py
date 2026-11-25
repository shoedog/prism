from flask import Flask


def create_app():
    app = Flask(__name__, static_folder=None)

    @app.get("/status")
    def status():
        return "ok"

    return app
