from django.urls import include, path, re_path

urlpatterns = [re_path(r"^ops/", include("reports.urls"))]
