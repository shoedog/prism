def f(status, reason, body, headers):
    try:
        from requests.models import Response
    except ImportError:
        return g(status, reason, body, dict(headers))
    response = Response()
    return response
