from typing import Annotated
from fastapi import FastAPI, Query
app = FastAPI()

def isolated():
    t = acquire()  # @SRC

@app.get("/view")
def view(value: str = Query()):
    return getattr(record, value)  # @SINK
