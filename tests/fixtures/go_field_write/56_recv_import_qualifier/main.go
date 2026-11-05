package fixture
import "pkg"
type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(x string) {
	pkg.Var.f = x
	pkg.Var.M()
}
