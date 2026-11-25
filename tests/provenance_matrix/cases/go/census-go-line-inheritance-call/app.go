package fixture

import ("fmt"; "os")

func read() {
    count := len("fixture")
    fmt.Println(count, os.Getenv("MODE"))
}
