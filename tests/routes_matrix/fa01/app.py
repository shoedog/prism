from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.get("/items/{id}")
def item(id: int):
    return {"id": id}
