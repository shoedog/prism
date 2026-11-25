variable "editions" { type = map(string) }
locals {
  selected = try(var.editions["current"], "preview")
  present  = can(var.editions["current"])
}
output "snapshot" {
  value = {
    data = [for selected in [local.selected] :
      selected
    ]
    flags = [for present in [local.present] :
      present
    ]
  }
}
