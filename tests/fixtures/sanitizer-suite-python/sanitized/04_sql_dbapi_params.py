from flask import Flask, request

app = Flask(__name__)

@app.route("/items")
def items(cursor):
    name = request.args.get("name")
    cursor.execute("SELECT * FROM items WHERE name = %s", (name,))
