import sqlite3

def get_conn():
    return sqlite3.connect(":memory:")

def load():
    conn = get_conn()
    row = conn.execute("select 1").fetchone()
    return row
