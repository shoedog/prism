from flask import Flask, url_for

app = Flask(__name__, static_folder=None)
app.add_url_rule("/media/<name>", endpoint="asset_link", methods=["GET"], build_only=True)


@app.get("/reference")
def reference():
    return {"href": url_for("asset_link", name="schedule.pdf")}
