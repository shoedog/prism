from django.http import HttpResponse
from django.urls import path, re_path


from reports.views import report


urlpatterns = [path("reports/", report)]
