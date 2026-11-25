from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.url = "/reports"


@app.get(app.url)
def reports():
    return []
