import sqlite3
def run():
    conn = sqlite3.connect(':memory:')
    cur = conn.cursor()
    cur.execute('select 1')
    row = cur.fetchone()
    count = conn.total_changes
    print(row, count)
