const vm = require("node:vm");
vm.runInThisContext(require("node:fs").readFileSync("defs.js", "utf8"));
require("./repo/T10_main.js");
