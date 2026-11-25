from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.api_route("/items", methods=["GET", "POST"])
def items():
    return []
