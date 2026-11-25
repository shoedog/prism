from django.http import HttpResponse
from django.urls import path
from django.views import View

class ExportView(View):
    def get(self, request):
        return HttpResponse("export")

    def post(self, request):
        return HttpResponse(status=202)

urlpatterns = [path("exports/", ExportView.as_view(), name="export")]
