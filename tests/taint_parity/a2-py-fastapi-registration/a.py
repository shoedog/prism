from typing import Annotated
from fastapi import FastAPI, Query
app = FastAPI()

def isolated():
    t = acquire()  # @SRC

def view(value: Annotated[str, Query()]):
    return getattr(record, value)  # @SINK
app.add_api_route("/view", view, methods=["GET"])
