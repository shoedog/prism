"""django integration example"""
from fastapi import FastAPI
app = FastAPI()

def isolated():
    t = acquire()  # @SRC

@app.get("/view")
def view(request: str):
    return getattr(record, request)  # @SINK
