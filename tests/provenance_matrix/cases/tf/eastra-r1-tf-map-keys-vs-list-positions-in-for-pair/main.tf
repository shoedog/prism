variable "editions" { type = map(string) }
variable "locales" { type = list(string) }
output "map_entries" {
  value = [for key, item in var.editions :
    [key, item]
  ]
}
output "numbered_entries" {
  value = [for key, item in var.locales :
    [key, item]
  ]
}
