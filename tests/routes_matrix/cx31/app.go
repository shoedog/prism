package fixture

import "github.com/gin-gonic/gin"

func fileOptions(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.OPTIONS("/files", fileOptions)
	return r
}
