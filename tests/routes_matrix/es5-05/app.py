from fastapi import FastAPI

app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)


@app.get("/reports/{report_id:int}")
def report(report_id: int):
    return {"report_id": report_id}


@app.get("/exports/{relative_path:path}")
def export(relative_path: str):
    return {"relative_path": relative_path}


@app.get("/examples/:name/{draft-name}")
def example():
    return {"kind": "literal"}
