from django.http import HttpResponse
from django.urls import path

def index(incoming):
    mode = incoming.headers.get("X-Preview")
    return HttpResponse(mode)

urlpatterns = [path("catalog/", index)]
