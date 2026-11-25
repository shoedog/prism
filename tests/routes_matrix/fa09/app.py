from fastapi import APIRouter, FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


admin = APIRouter(tags=["admin"])
reports = APIRouter(tags=["reports"])


@reports.get("/summary")
def summary():
    return {"total": 0}


admin.include_router(reports, prefix="/reports")
app.include_router(admin, prefix="/admin")
