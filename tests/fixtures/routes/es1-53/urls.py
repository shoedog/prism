from django.http import HttpResponse
from django.urls import path, register_converter

class UpperConverter:
    regex = "[A-Z]+"

    def to_python(self, value):
        return value

    def to_url(self, value):
        return value

register_converter(UpperConverter, "upper")

def asset(request, key):
    return HttpResponse(key)

urlpatterns = [
    path("asset/<path:key>/", asset),
    path("label/<upper:key>/", asset),
]
