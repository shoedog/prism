from django.http import HttpResponse
from django.urls import path as url

def health(request):
    return HttpResponse("ok")

unused = url("draft/", health)
urlpatterns = [url("health/", health)]
