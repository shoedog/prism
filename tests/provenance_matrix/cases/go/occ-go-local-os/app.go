package fixture

import ("os"; "net/http")

func currentMode() string { return os.Getenv("MODE") }

func show(r *http.Request) string {
    os := r.FormValue("os")
    return os
}
