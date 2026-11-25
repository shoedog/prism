variable "editions" { type = list(string) }
locals {
  grouped = { for edition in var.editions : "catalog" => edition... }
}
output "editions" {
  value = [for entry in lookup(local.grouped, "catalog", []) :
    entry
  ]
}
