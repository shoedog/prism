from flask import Flask, request
from sqlalchemy import text

app = Flask(__name__)

@app.route("/items")
def items(session):
    name = request.args.get("name")
    session.execute(text(f"SELECT * FROM items WHERE name = '{name}'"))
