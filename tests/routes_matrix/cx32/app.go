package fixture

import "github.com/gin-gonic/gin"

func updateProfile(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.PATCH("/profile", updateProfile)
	return r
}
