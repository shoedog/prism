from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class Item(BaseModel):
    name: str

@app.post("/items")
def create_item(item: Item, cursor):
    cursor.execute("SELECT * FROM items WHERE name = %s", (item.name,))
