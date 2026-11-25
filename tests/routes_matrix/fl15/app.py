from flask import Flask


class StatusSite:
    def __init__(self):
        self.app = Flask(__name__, static_folder=None)

        @self.app.route("/x")
        def status():
            return {"status": "ok"}


site = StatusSite()
app = site.app
