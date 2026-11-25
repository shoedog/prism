from fastapi import FastAPI, Request
app = FastAPI()

@app.get("/preview")
def current(request: Request):
    return request
