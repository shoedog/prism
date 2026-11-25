import sqlite3
def read():
    db = sqlite3.connect(':memory:')
    cursor = db.cursor()
    cursor.execute('select 1')
    rows = cursor.fetchall()
    count = cursor.rowcount
    return rows, count
