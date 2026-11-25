from flask import Flask

app = Flask(__name__, static_folder=None)


class Panel:
    def status(self):
        return "ok"


panel = Panel()


app.add_url_rule("/status", methods=["GET"], view_func=panel.status)
