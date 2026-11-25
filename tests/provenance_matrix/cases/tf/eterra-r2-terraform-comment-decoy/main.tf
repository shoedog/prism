locals {
  label = "fixed" # var.secret
}
output "label" {
  value = local.label
}
