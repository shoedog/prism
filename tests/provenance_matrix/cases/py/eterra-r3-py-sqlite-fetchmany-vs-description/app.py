import sqlite3
def run():
    conn = sqlite3.connect(':memory:')
    cur = conn.cursor()
    cur.execute('select 1')
    rows = cur.fetchmany(1)
    meta = cur.description
    return rows, meta
