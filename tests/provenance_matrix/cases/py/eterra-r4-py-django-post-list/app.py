from django.http import HttpRequest
def view(request: HttpRequest):
    tags = request.POST.getlist('tag')
    return tags
