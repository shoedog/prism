package fixture

import ("net/http"; "strconv")

func read(r *http.Request) (int, error) {
    id, err := strconv.Atoi(r.FormValue("id"))
    return id, err
}
