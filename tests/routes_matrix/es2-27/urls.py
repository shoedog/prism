from django.http import HttpResponse
from django.urls import path as url_path


def status(request):
    return HttpResponse("ready")


def path(pattern, callback):
    return {"pattern": pattern, "callback": callback}


preview = path("preview/", status)
urlpatterns = [url_path("status/", status)]
