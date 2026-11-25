from django.http import HttpResponse
from django.urls import path

def show(incoming):
    label = incoming.META.get("HTTP_X_LABEL")
    route = incoming.resolver_match.route
    return HttpResponse((label, route))

urlpatterns = [path("catalog/", show)]
