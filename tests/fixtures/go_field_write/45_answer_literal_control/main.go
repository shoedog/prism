package fixture
import "html/template"
type Request struct { Username string }
type User struct { BioHTML string }
func scenario(input string) {
 username := input
 req := &Request{Username: username}
 userinfo, err := lookup(req)
 if err != nil { return }
 _, err = search(req.Username)
 if err != nil { return }
 render(template.HTML(userinfo.BioHTML))
}
