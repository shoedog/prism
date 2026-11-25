from django.urls import include, path

urlpatterns = [path("reports/", include("reports", namespace="staff_reports"))]
