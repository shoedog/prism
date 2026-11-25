package moonspindle

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	r.UseRawPath = true
	r.GET("/fletch/wax%2Fpith", func(c *gin.Context) { c.String(200, "wax") })
	r.GET("/fletch/wax/pith", func(c *gin.Context) { c.String(200, "pith") })
}
