from fastapi import FastAPI
from fastapi.routing import APIRoute


def catalog():
    return {'catalog': []}


app = FastAPI(
    routes=[APIRoute('/catalog', catalog, methods=['GET'])],
    docs_url=None,
    redoc_url=None,
    openapi_url=None,
)


@app.get('/ready')
def ready():
    return {'ready': True}
