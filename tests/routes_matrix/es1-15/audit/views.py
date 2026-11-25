from flask import Blueprint

events = Blueprint("events", __name__)

@events.get("/events")
def recent():
    return "events"
