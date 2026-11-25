package moonspindle

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	r.Group("/fletch").StaticFile("/glint.txt", "./glint.txt")
}
