from fastapi import BackgroundTasks, FastAPI, Query
app = FastAPI()

@app.get("/editions")
def list_editions(work: BackgroundTasks, title: str = Query()):
    work.add_task(print, title)
    return {"ok": True}
