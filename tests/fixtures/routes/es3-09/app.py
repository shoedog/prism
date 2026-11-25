from fastapi import FastAPI

source = FastAPI(title='Reports', docs_url=None, redoc_url=None, openapi_url=None)


@source.get('/recent')
def recent():
    return {'recent': []}


app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
app.include_router(source.router, prefix='/reports')
