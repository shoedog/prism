package main

import (
	"os"
	"path/filepath"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Param("file")
	base := filepath.Base(name)
	_, _ = os.ReadFile(base)
}
