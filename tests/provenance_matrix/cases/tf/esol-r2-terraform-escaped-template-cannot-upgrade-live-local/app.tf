locals {
  caption = "Catalog"
}
output "banner" {
  value = "$${var.fake}: ${local.caption}"
}
