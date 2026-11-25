package fixture

import ("net/http"; "io")

func read(url string) io.ReadCloser {
    resp, _ := http.Get(url)
    stream := resp.Body
    return stream
}
