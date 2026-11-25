from sqlalchemy import create_engine, text
def labels(url):
    engine = create_engine(url)
    with engine.connect() as connection:
        result = connection.execute(text("select title from catalog limit 1"))
        headers = tuple(result.keys())
        record = result.mappings().one()
        title = record["title"]
        return title, headers
