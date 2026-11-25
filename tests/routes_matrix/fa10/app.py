from fastapi import APIRouter, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


reports = APIRouter()
P = "/team"


@reports.get("/summary")
def summary():
    return {"total": 0}


app.include_router(reports, prefix=P)
