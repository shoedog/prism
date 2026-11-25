from fastapi import Depends, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


def authenticate():
    return True


@app.get("/invoices", dependencies=[Depends(authenticate)])
def invoices():
    return []
