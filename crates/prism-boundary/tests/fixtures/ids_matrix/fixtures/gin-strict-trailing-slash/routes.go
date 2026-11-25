package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	r.RedirectTrailingSlash = false
	r.GET("/mugs", glaze)
	r.GET("/mugs/", glazeMatte)
}

func glaze(c *gin.Context)      { c.String(200, "glaze") }
func glazeMatte(c *gin.Context) { c.String(200, "matte") }
