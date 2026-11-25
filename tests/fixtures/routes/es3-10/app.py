from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.api_route('/reserved', methods=[], operation_id='reserved')
def reserved():
    return {}


@app.api_route('/ready')
def ready():
    return {'ready': True}
