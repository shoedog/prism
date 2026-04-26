package main

import (
	"os"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	_ = os.WriteFile(name, []byte("data"), 0644)
}
