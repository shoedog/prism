import psycopg

async def load(dsn):
    async with await psycopg.AsyncConnection.connect(dsn) as conn:
        async with conn.cursor() as cur:
            await cur.execute("select title from catalog")
            record = await cur.fetchone()
            return record
