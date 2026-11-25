from django.http import HttpResponse
from django.urls import path


class OfficeDirectory:
    def __init__(self, get_response):
        self.get_response = get_response

    def __call__(self, request):
        if request.headers.get("X-Directory-Edition") == "office":
            request.urlconf = "office"
        return self.get_response(request)


def office_directory(request):
    return HttpResponse("office directory")


urlpatterns = [path("directory/", office_directory)]
