package fixture

import "os"

func show() {
    println := os.Getenv("MODE")
    _ = println
    {
        println := func() {}
        println()
    }
    printlnValue := len(println)
    _ = printlnValue
}
