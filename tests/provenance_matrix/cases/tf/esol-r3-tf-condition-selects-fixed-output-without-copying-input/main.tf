variable "enabled" { type = bool }

locals {
  badge = var.enabled ? "enabled" : "disabled"
}

output "badge" {
  value = local.badge
}
