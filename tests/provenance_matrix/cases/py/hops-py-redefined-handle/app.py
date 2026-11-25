import sqlite3

def load():
    conn = sqlite3.connect(":memory:")
    conn = sqlite3.connect(":memory:")
    row = conn.execute("select 1").fetchone()
    return row
