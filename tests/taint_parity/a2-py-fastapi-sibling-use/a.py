from typing import Annotated
from fastapi import FastAPI, Query
app = FastAPI()

def isolated():
    t = acquire()  # @SRC
    observe(t)

@app.get("/view")
def view(value: Annotated[str, Query()]):
    return getattr(record, value)  # @SINK
