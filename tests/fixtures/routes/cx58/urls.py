from django.http import HttpResponse
from django.urls import path, re_path


class Reports:
    def index(self, request):
        return HttpResponse("ok")


reports = Reports()


urlpatterns = [path("reports/", reports.index)]
