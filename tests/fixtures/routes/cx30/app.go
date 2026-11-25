package fixture

import "github.com/gin-gonic/gin"

func archiveHead(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.HEAD("/archive", archiveHead)
	return r
}
