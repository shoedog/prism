package main
import "os/exec"
import "sync"
var wg sync.WaitGroup
func run(x string) {
    defer wg.Done()
    exec.Command(x) // @SINK
}
func main() {
    t := acquire() // @SRC
    wg.Add(1)
    go run(t)
    wg.Wait()
}
func acquire() string { return "value" }
