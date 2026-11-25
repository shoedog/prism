import sqlalchemy as sa
from sqlalchemy.orm import Session

def load():
    engine = sa.create_engine("sqlite://")
    with Session(engine) as session:
        labels = session.scalars(sa.text("select 'desk' as label")).all()
        return labels
