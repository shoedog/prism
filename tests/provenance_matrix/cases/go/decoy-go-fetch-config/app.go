package fixture

func fetchConfig(name string) bool { return name == "show" }

func show() bool {
    state := fetchConfig("show-mode")
    return state
}
