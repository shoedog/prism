from django.http import HttpResponse
from django.urls import path


def published_summary(request):
    return HttpResponse("published summary")


urlpatterns = [path("summary/", published_summary)]
