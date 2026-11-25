locals {
  edition = jsondecode(file("${path.module}/settings.json")).edition
  present = fileexists("${path.module}/settings.json")
}
output "deployment" {
  value = {
    names = [for label in [local.edition] :
      label
    ]
    flags = [for exists in [local.present] :
      exists
    ]
  }
}
