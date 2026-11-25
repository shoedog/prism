from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


def update_item():
    return {"updated": True}


app.add_api_route("/x", update_item, methods=["PUT"])
