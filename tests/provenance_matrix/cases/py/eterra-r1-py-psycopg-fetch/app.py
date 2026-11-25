import psycopg
def run():
    conn = psycopg.connect('dbname=test')
    cur = conn.cursor()
    cur.execute('select 1')
    row = cur.fetchone()
    print(row)
