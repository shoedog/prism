import sqlite3

def read(path):
    with sqlite3.connect(path) as db:
        old = db.cursor()
        db.row_factory = lambda cursor, fields: "preview"
        fresh = db.cursor()
        row = old.execute("select title from books limit 1").fetchone()
        if row is None:
            return
        replacement = fresh.execute("select count(*) from books").fetchone()
        title = row[0]
        return title, replacement
