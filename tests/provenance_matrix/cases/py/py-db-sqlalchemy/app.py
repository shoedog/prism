from sqlalchemy import create_engine
from sqlalchemy.orm import Session

def load(model):
    engine = create_engine("sqlite://")
    session = Session(engine)
    row = session.query(model).first()
    return row
