package fixture

import "os"

func preview(label string) string {
  if label := os.Getenv("PREVIEW_LABEL"); label != "" {
    return label
  }
  return label
}
