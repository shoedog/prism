from flask import Flask

app = Flask(__name__, static_folder=None)


class Sink:
    def get(self, path):
        def identity(view):
            return view

        return identity


def configure_preview():
    app = Sink()

    @app.get("/x")
    def preview():
        return "preview"


configure_preview()
