from typing import Annotated
from fastapi import FastAPI, Query
app, *rest = FastAPI(), 0, 1

def isolated():
    t = acquire()  # @SRC

@app.get("/view")
def view(value: Annotated[str, Query()]):
    return getattr(record, value)  # @SINK
