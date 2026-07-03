from fastapi import FastAPI

app = FastAPI()


@app.get("/x")
def handler():
    return "ok"
