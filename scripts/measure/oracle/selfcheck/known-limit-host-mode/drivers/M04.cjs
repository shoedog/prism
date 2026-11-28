const {readFileSync} = require("node:fs");
const {runInThisContext} = require("node:vm");
for (const filename of process.argv.slice(2).length ? process.argv.slice(2) : ["defs.js", "M04_main.js"]) {
runInThisContext(readFileSync(filename,"utf8"), {filename});
}
