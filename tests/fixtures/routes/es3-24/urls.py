from django.urls import include, re_path

urlpatterns = [re_path(r'^desk/$', include('desk_urls'))]
