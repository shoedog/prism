package fixture

type Item struct { Label string }

func show(obj Item) string {
    return obj.Label
}
