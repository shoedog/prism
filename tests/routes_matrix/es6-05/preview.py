from django.http import HttpResponse
from django.urls import path


def preview_summary(request):
    return HttpResponse("preview summary")


urlpatterns = [path("summary/", preview_summary)]
