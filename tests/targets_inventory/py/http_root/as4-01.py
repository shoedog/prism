try:
    from httpx import get
except ImportError:
    def fetch(u):
        from requests import get
        return get(u)
else:
    def fetch(u):
        return get(u)
