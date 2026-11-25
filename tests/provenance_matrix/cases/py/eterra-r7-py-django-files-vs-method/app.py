from django.http import HttpRequest

def upload(request: HttpRequest):
    document = request.FILES['document']
    method = request.method
    return document, method
