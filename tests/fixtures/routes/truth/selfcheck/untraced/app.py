from fastapi import FastAPI

app = FastAPI()


@app.get("/real")
def real():
    return None
