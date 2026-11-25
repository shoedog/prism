from django.http import HttpResponse
from django.urls import path

app_name = "reports"

def daily(request):
    return HttpResponse("daily")

urlpatterns = [path("daily/", daily)]
