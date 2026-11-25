package catalog
import (
    "net/http"
    "net/http/httputil"
    "net/url"
)
func proxy(backend *url.URL, emit func(string)) http.Handler {
    forward := httputil.NewSingleHostReverseProxy(backend)
    forward.ModifyResponse = func(incoming *http.Response) error {
        tag := incoming.Header.Get("X-Edition")
        emit(tag)
        return nil
    }
    return forward
}
