package fixture
import "os/exec"
type A struct { b, c string }
type S struct { f, g string; a A }

func scenario() {
	x := source()
	cmd := exec.Command("ls")
	cmd.Dir = x
	cmd.Run()
}
