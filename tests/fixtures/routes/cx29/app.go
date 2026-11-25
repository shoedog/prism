package fixture

import "github.com/gin-gonic/gin"

func removeNote(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.DELETE("/notes/:id", removeNote)
	return r
}
