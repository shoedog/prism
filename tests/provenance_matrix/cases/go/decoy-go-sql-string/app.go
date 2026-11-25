package fixture

import "strings"

func load() string {
    label := strings.ToUpper(`user_id = $1`)
    return label
}
